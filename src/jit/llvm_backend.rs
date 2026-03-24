#![allow(unsafe_op_in_unsafe_fn)]

use crate::runtime::{FunctionType, Module, NumType, Result, ValType, WasmError, WasmValue};
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;

#[cfg(feature = "llvm-jit")]
use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
#[cfg(feature = "llvm-jit")]
use llvm_sys::core::*;
#[cfg(feature = "llvm-jit")]
use llvm_sys::orc2::lljit::{
    LLVMOrcCreateLLJIT, LLVMOrcCreateLLJITBuilder, LLVMOrcDisposeLLJIT, LLVMOrcDisposeLLJITBuilder,
    LLVMOrcLLJITAddLLVMIRModule, LLVMOrcLLJITBuilderSetJITTargetMachineBuilder,
    LLVMOrcLLJITGetMainJITDylib, LLVMOrcLLJITLookup, LLVMOrcLLJITRef,
};
#[cfg(feature = "llvm-jit")]
use llvm_sys::orc2::{
    LLVMOrcCreateNewThreadSafeContext, LLVMOrcCreateNewThreadSafeModule,
    LLVMOrcDisposeThreadSafeContext, LLVMOrcDisposeThreadSafeModule, LLVMOrcExecutorAddress,
    LLVMOrcJITDylibRef, LLVMOrcJITTargetMachineBuilderDetectHost,
    LLVMOrcJITTargetMachineBuilderRef, LLVMOrcThreadSafeContextGetContext,
    LLVMOrcThreadSafeContextRef,
};
#[cfg(feature = "llvm-jit")]
use llvm_sys::prelude::*;
#[cfg(feature = "llvm-jit")]
use llvm_sys::target::{
    LLVM_InitializeNativeAsmParser, LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget,
};

#[cfg(feature = "llvm-jit")]
use super::llvm_runtime::{clear_trap, take_trap};
use super::wasm_to_llvm::WasmToLlvmTranslator;

struct CompiledFunction {
    entry_point: *const u8,
    func_type: FunctionType,
}

/// LLVM-based JIT compiler for WebAssembly modules.
///
/// Uses LLVM 17's ORC JIT API to compile WASM functions to native code
/// with optimisations like loop vectorisation, constant propagation, and inlining.
///
/// # Thread Safety
///
/// The JIT instance can be shared across threads, but compiled functions must
/// be invoked with the memory context set for the calling thread using
/// [`set_memory_context`][crate::jit::set_memory_context].
///
/// # Example
///
/// ```ignore
/// let mut jit = LlvmJit::new("my_module")?;
/// let compiled = jit.compile_module(&module)?;
/// let result = jit.invoke_function(0, &[WasmValue::I32(42)])?;
/// ```
pub struct LlvmJit {
    #[cfg(feature = "llvm-jit")]
    thread_safe_context: LLVMOrcThreadSafeContextRef,
    #[cfg(feature = "llvm-jit")]
    lljit: LLVMOrcLLJITRef,
    #[cfg(feature = "llvm-jit")]
    main_dylib: LLVMOrcJITDylibRef,
    compiled_functions: HashMap<u32, CompiledFunction>,
    #[cfg(feature = "llvm-jit")]
    helpers_registered: bool,
}

/// A compiled WASM function ready for execution.
#[derive(Debug, Clone)]
pub struct CompiledLlvmFunction {
    /// The function index in the original WASM module.
    pub func_idx: u32,
    /// Raw pointer to the compiled function entry point.
    pub entry_point: *const u8,
}

impl LlvmJit {
    #[cfg(feature = "llvm-jit")]
    fn validate_module_compatibility(module: &Module) -> Result<()> {
        for (type_idx, func_type) in module.types.iter().enumerate() {
            if func_type.results.len() > 1 {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support multi-value function type {}",
                    type_idx
                )));
            }
            if func_type
                .params
                .iter()
                .chain(func_type.results.iter())
                .any(|value_type| matches!(value_type, ValType::Ref(_)))
            {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support reference-typed function type {}",
                    type_idx
                )));
            }
        }

        for (func_idx, func) in module.funcs.iter().enumerate() {
            if func
                .locals
                .iter()
                .any(|local| matches!(local.type_, ValType::Ref(_)))
            {
                return Err(WasmError::Runtime(format!(
                    "LLVM JIT does not support reference-typed locals in function {}",
                    func_idx
                )));
            }
        }

        Ok(())
    }

    /// Creates a new LLVM JIT instance.
    ///
    /// Initialises the LLVM native target, ASM printer, and ASM parser,
    /// then creates an ORC LLJIT instance for compilation.
    ///
    /// # Errors
    ///
    /// Returns an error if LLVM initialisation fails or the JIT cannot be created.
    #[cfg(feature = "llvm-jit")]
    pub fn new(_module_name: &str) -> Result<Self> {
        unsafe {
            if LLVM_InitializeNativeTarget() != 0 {
                return Err(WasmError::Runtime(
                    "Failed to initialize native target".to_string(),
                ));
            }
            if LLVM_InitializeNativeAsmPrinter() != 0 {
                return Err(WasmError::Runtime(
                    "Failed to initialize native ASM printer".to_string(),
                ));
            }
            if LLVM_InitializeNativeAsmParser() != 0 {
                return Err(WasmError::Runtime(
                    "Failed to initialize native ASM parser".to_string(),
                ));
            }

            let thread_safe_context = LLVMOrcCreateNewThreadSafeContext();
            if thread_safe_context.is_null() {
                return Err(WasmError::Runtime(
                    "Failed to create thread-safe context".to_string(),
                ));
            }

            let mut jtmb: LLVMOrcJITTargetMachineBuilderRef = ptr::null_mut();
            let result = LLVMOrcJITTargetMachineBuilderDetectHost(&mut jtmb);
            if !result.is_null() {
                LLVMOrcDisposeThreadSafeContext(thread_safe_context);
                return Err(WasmError::Runtime(
                    "Failed to detect host target".to_string(),
                ));
            }

            let builder = LLVMOrcCreateLLJITBuilder();
            if builder.is_null() {
                LLVMOrcDisposeThreadSafeContext(thread_safe_context);
                return Err(WasmError::Runtime(
                    "Failed to create LLJIT builder".to_string(),
                ));
            }

            LLVMOrcLLJITBuilderSetJITTargetMachineBuilder(builder, jtmb);

            let mut lljit: LLVMOrcLLJITRef = ptr::null_mut();
            let result = LLVMOrcCreateLLJIT(&mut lljit, builder);
            if !result.is_null() {
                LLVMOrcDisposeLLJITBuilder(builder);
                LLVMOrcDisposeThreadSafeContext(thread_safe_context);
                return Err(WasmError::Runtime(
                    "Failed to create LLJIT instance".to_string(),
                ));
            }

            let main_dylib = LLVMOrcLLJITGetMainJITDylib(lljit);

            Ok(Self {
                thread_safe_context,
                lljit,
                main_dylib,
                compiled_functions: HashMap::new(),
                helpers_registered: false,
            })
        }
    }

    #[cfg(not(feature = "llvm-jit"))]
    pub fn new(_module_name: &str) -> Result<Self> {
        Err(WasmError::Runtime(
            "LLVM JIT not available: compile with --features llvm-jit".to_string(),
        ))
    }

    #[cfg(feature = "llvm-jit")]
    pub fn register_runtime_helpers(&mut self) -> Result<()> {
        use super::llvm_runtime::*;

        macro_rules! register_helper {
            ($name:expr, $func:expr) => {
                self.register_host_function($name, $func as *const u8)?;
            };
        }

        register_helper!("llvm_jit_i32_load", llvm_jit_i32_load as *const u8);
        register_helper!("llvm_jit_i64_load", llvm_jit_i64_load as *const u8);
        register_helper!("llvm_jit_f32_load", llvm_jit_f32_load as *const u8);
        register_helper!("llvm_jit_f64_load", llvm_jit_f64_load as *const u8);
        register_helper!("llvm_jit_i32_load8_s", llvm_jit_i32_load8_s as *const u8);
        register_helper!("llvm_jit_i32_load8_u", llvm_jit_i32_load8_u as *const u8);
        register_helper!("llvm_jit_i32_load16_s", llvm_jit_i32_load16_s as *const u8);
        register_helper!("llvm_jit_i32_load16_u", llvm_jit_i32_load16_u as *const u8);
        register_helper!("llvm_jit_i64_load8_s", llvm_jit_i64_load8_s as *const u8);
        register_helper!("llvm_jit_i64_load8_u", llvm_jit_i64_load8_u as *const u8);
        register_helper!("llvm_jit_i64_load16_s", llvm_jit_i64_load16_s as *const u8);
        register_helper!("llvm_jit_i64_load16_u", llvm_jit_i64_load16_u as *const u8);
        register_helper!("llvm_jit_i64_load32_s", llvm_jit_i64_load32_s as *const u8);
        register_helper!("llvm_jit_i64_load32_u", llvm_jit_i64_load32_u as *const u8);
        register_helper!("llvm_jit_i32_store", llvm_jit_i32_store as *const u8);
        register_helper!("llvm_jit_i64_store", llvm_jit_i64_store as *const u8);
        register_helper!("llvm_jit_f32_store", llvm_jit_f32_store as *const u8);
        register_helper!("llvm_jit_f64_store", llvm_jit_f64_store as *const u8);
        register_helper!("llvm_jit_i32_store8", llvm_jit_i32_store8 as *const u8);
        register_helper!("llvm_jit_i32_store16", llvm_jit_i32_store16 as *const u8);
        register_helper!("llvm_jit_i64_store8", llvm_jit_i64_store8 as *const u8);
        register_helper!("llvm_jit_i64_store16", llvm_jit_i64_store16 as *const u8);
        register_helper!("llvm_jit_i64_store32", llvm_jit_i64_store32 as *const u8);
        register_helper!("llvm_jit_i32_div_s", llvm_jit_i32_div_s as *const u8);
        register_helper!("llvm_jit_i32_div_u", llvm_jit_i32_div_u as *const u8);
        register_helper!("llvm_jit_i32_rem_s", llvm_jit_i32_rem_s as *const u8);
        register_helper!("llvm_jit_i32_rem_u", llvm_jit_i32_rem_u as *const u8);
        register_helper!("llvm_jit_i64_div_s", llvm_jit_i64_div_s as *const u8);
        register_helper!("llvm_jit_i64_div_u", llvm_jit_i64_div_u as *const u8);
        register_helper!("llvm_jit_i64_rem_s", llvm_jit_i64_rem_s as *const u8);
        register_helper!("llvm_jit_i64_rem_u", llvm_jit_i64_rem_u as *const u8);
        register_helper!("llvm_jit_f32_min", llvm_jit_f32_min as *const u8);
        register_helper!("llvm_jit_f64_min", llvm_jit_f64_min as *const u8);
        register_helper!("llvm_jit_has_trap", llvm_jit_has_trap as *const u8);
        register_helper!(
            "llvm_jit_trap_unreachable",
            llvm_jit_trap_unreachable as *const u8
        );
        register_helper!("llvm_jit_call_import", llvm_jit_call_import as *const u8);

        Ok(())
    }

    /// Compiles all defined functions in a WASM module to native code.
    ///
    /// Each function is translated to LLVM IR, optimised, and compiled to
    /// machine code. The compiled functions are stored internally for later
    /// invocation via [`invoke_function`](Self::invoke_function).
    ///
    /// # Arguments
    ///
    /// * `module` - The WASM module containing functions to compile.
    ///
    /// # Returns
    ///
    /// A vector of [`CompiledLlvmFunction`] entries, one for each compiled function.
    ///
    /// # Errors
    ///
    /// Returns an error if translation or compilation fails for any function.
    #[cfg(feature = "llvm-jit")]
    pub fn compile_module(&mut self, module: &Module) -> Result<Vec<CompiledLlvmFunction>> {
        Self::validate_module_compatibility(module)?;

        if !self.helpers_registered {
            self.register_runtime_helpers()?;
            self.helpers_registered = true;
        }

        unsafe {
            let mut compiled = Vec::new();
            let mut pending_functions = Vec::new();

            let import_func_count = module
                .imports
                .iter()
                .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
                .count() as u32;

            for func_idx in import_func_count..(import_func_count + module.funcs.len() as u32) {
                let local_idx = func_idx - import_func_count;
                if let Some(func) = module.defined_func_at(local_idx) {
                    let func_type = module
                        .func_type(func_idx)
                        .ok_or_else(|| {
                            WasmError::Runtime(format!("Function type not found for {}", func_idx))
                        })?
                        .clone();

                    let ts_context = LLVMOrcCreateNewThreadSafeContext();
                    let context = LLVMOrcThreadSafeContextGetContext(ts_context);

                    let mut translator = WasmToLlvmTranslator::new(context)?;
                    let translate_result = translator.translate_function(func, func_idx, module);

                    let llvm_module = match translate_result {
                        Ok(result) => result,
                        Err(e) => {
                            LLVMOrcDisposeThreadSafeContext(ts_context);
                            return Err(e);
                        }
                    };

                    self.add_module(llvm_module, ts_context)?;
                    pending_functions.push((func_idx, func_type));
                }
            }

            for (func_idx, func_type) in pending_functions {
                let entry_name = format!("wasm_entry_{}", func_idx);
                let entry_point = self.lookup_symbol(&entry_name)?;

                self.compiled_functions.insert(
                    func_idx,
                    CompiledFunction {
                        entry_point,
                        func_type,
                    },
                );
                compiled.push(CompiledLlvmFunction {
                    func_idx,
                    entry_point,
                });
            }

            Ok(compiled)
        }
    }

    #[cfg(feature = "llvm-jit")]
    fn add_module(
        &mut self,
        llvm_module: LLVMModuleRef,
        ts_context: LLVMOrcThreadSafeContextRef,
    ) -> Result<()> {
        unsafe {
            let mut error_msg: *mut i8 = ptr::null_mut();
            let result = LLVMVerifyModule(
                llvm_module,
                LLVMVerifierFailureAction::LLVMPrintMessageAction,
                &mut error_msg,
            );
            if result != 0 {
                if !error_msg.is_null() {
                    LLVMDisposeMessage(error_msg);
                }
                LLVMDisposeModule(llvm_module);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(WasmError::Runtime(
                    "LLVM module verification failed".to_string(),
                ));
            }

            let thread_safe_module = LLVMOrcCreateNewThreadSafeModule(llvm_module, ts_context);

            if thread_safe_module.is_null() {
                LLVMDisposeModule(llvm_module);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(WasmError::Runtime(
                    "Failed to create thread-safe module".to_string(),
                ));
            }

            let result =
                LLVMOrcLLJITAddLLVMIRModule(self.lljit, self.main_dylib, thread_safe_module);
            if !result.is_null() {
                LLVMOrcDisposeThreadSafeModule(thread_safe_module);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(WasmError::Runtime(
                    "Failed to add LLVM IR module".to_string(),
                ));
            }

            LLVMOrcDisposeThreadSafeContext(ts_context);
            Ok(())
        }
    }

    #[cfg(feature = "llvm-jit")]
    fn lookup_symbol(&self, symbol_name: &str) -> Result<*const u8> {
        unsafe {
            let symbol_name_c = CString::new(symbol_name)
                .map_err(|_| WasmError::Runtime("Symbol name contains NUL byte".to_string()))?;
            let mut symbol: LLVMOrcExecutorAddress = 0;
            let result = LLVMOrcLLJITLookup(self.lljit, &mut symbol, symbol_name_c.as_ptr());
            if !result.is_null() || symbol == 0 {
                return Err(WasmError::Runtime(format!(
                    "Failed to lookup symbol {}",
                    symbol_name
                )));
            }
            Ok(symbol as *const u8)
        }
    }

    pub fn get_function_entry(&self, func_idx: u32) -> Option<*const u8> {
        self.compiled_functions
            .get(&func_idx)
            .map(|cf| cf.entry_point)
    }

    #[cfg(feature = "llvm-jit")]
    pub fn register_host_function(&mut self, name: &str, addr: *const u8) -> Result<()> {
        unsafe {
            use llvm_sys::orc2::lljit::LLVMOrcLLJITMangleAndIntern;
            use llvm_sys::orc2::{
                LLVMJITEvaluatedSymbol, LLVMJITSymbolFlags, LLVMOrcAbsoluteSymbols,
                LLVMOrcCSymbolMapPair, LLVMOrcJITDylibDefine,
            };

            let name_c = CString::new(name).map_err(|_| {
                WasmError::Runtime("Host function name contains NUL byte".to_string())
            })?;
            let symbol_name = LLVMOrcLLJITMangleAndIntern(self.lljit, name_c.as_ptr());

            let symbol = LLVMJITEvaluatedSymbol {
                Address: addr as u64,
                Flags: LLVMJITSymbolFlags {
                    GenericFlags: 0,
                    TargetFlags: 0,
                },
            };

            let mut pair = LLVMOrcCSymbolMapPair {
                Name: symbol_name,
                Sym: symbol,
            };

            let mu = LLVMOrcAbsoluteSymbols(&mut pair, 1);
            if mu.is_null() {
                return Err(WasmError::Runtime(format!(
                    "Failed to create absolute symbols for {}",
                    name
                )));
            }

            let result = LLVMOrcJITDylibDefine(self.main_dylib, mu);
            if !result.is_null() {
                return Err(WasmError::Runtime(format!(
                    "Failed to register host function {}",
                    name
                )));
            }

            Ok(())
        }
    }

    fn pack_arg(value: &WasmValue) -> Result<u64> {
        match value {
            WasmValue::I32(v) => Ok(*v as u32 as u64),
            WasmValue::I64(v) => Ok(*v as u64),
            WasmValue::F32(v) => Ok(v.to_bits() as u64),
            WasmValue::F64(v) => Ok(v.to_bits()),
            _ => Err(WasmError::Runtime(
                "LLVM JIT currently supports numeric arguments only".to_string(),
            )),
        }
    }

    fn unpack_result(raw: u64, value_type: ValType) -> Result<WasmValue> {
        match value_type {
            ValType::Num(NumType::I32) => Ok(WasmValue::I32(raw as u32 as i32)),
            ValType::Num(NumType::I64) => Ok(WasmValue::I64(raw as i64)),
            ValType::Num(NumType::F32) => Ok(WasmValue::F32(f32::from_bits(raw as u32))),
            ValType::Num(NumType::F64) => Ok(WasmValue::F64(f64::from_bits(raw))),
            _ => Err(WasmError::Runtime(
                "LLVM JIT currently supports numeric results only".to_string(),
            )),
        }
    }

    /// Invokes a compiled function with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `func_idx` - The function index (adjusted for imports).
    /// * `args` - Arguments to pass to the function.
    ///
    /// # Returns
    ///
    /// The function's return values as a vector of [`WasmValue`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The function was not compiled
    /// - Argument count or types don't match the function signature
    /// - The function signature is not supported (e.g., multi-value returns)
    ///
    /// # Safety
    ///
    /// The caller must ensure the execution context is set for the current thread.
    #[cfg(feature = "llvm-jit")]
    pub fn invoke_function(&self, func_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let compiled = self
            .compiled_functions
            .get(&func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("Function {} not compiled", func_idx)))?;

        let entry_point = compiled.entry_point;
        let func_type = &compiled.func_type;

        if args.len() != func_type.params.len() {
            return Err(WasmError::Runtime(format!(
                "Argument count mismatch: expected {}, got {}",
                func_type.params.len(),
                args.len()
            )));
        }

        for (idx, (arg, expected_type)) in args.iter().zip(func_type.params.iter()).enumerate() {
            if arg.val_type() != *expected_type {
                return Err(WasmError::Runtime(format!(
                    "Argument {} type mismatch: expected {:?}, got {:?}",
                    idx,
                    expected_type,
                    arg.val_type()
                )));
            }
        }

        let mut packed_args = Vec::with_capacity(args.len());
        for arg in args {
            packed_args.push(Self::pack_arg(arg)?);
        }
        let mut packed_results = vec![0u64; func_type.results.len().max(1)];

        unsafe {
            clear_trap();
            let func: extern "C" fn(*const u64, *mut u64) = std::mem::transmute(entry_point);
            func(packed_args.as_ptr(), packed_results.as_mut_ptr());

            if let Some(code) = take_trap() {
                return Err(WasmError::Trap(code));
            }
        }

        let mut results = Vec::with_capacity(func_type.results.len());
        for (idx, result_type) in func_type.results.iter().copied().enumerate() {
            results.push(Self::unpack_result(packed_results[idx], result_type)?);
        }

        Ok(results)
    }
}

#[cfg(feature = "llvm-jit")]
impl Drop for LlvmJit {
    fn drop(&mut self) {
        unsafe {
            if !self.lljit.is_null() {
                LLVMOrcDisposeLLJIT(self.lljit);
            }
            if !self.thread_safe_context.is_null() {
                LLVMOrcDisposeThreadSafeContext(self.thread_safe_context);
            }
        }
    }
}

#[cfg(not(feature = "llvm-jit"))]
impl Drop for LlvmJit {
    fn drop(&mut self) {}
}

#[cfg(all(test, feature = "llvm-jit"))]
mod tests {
    use super::*;
    use crate::runtime::{Func, Module, NumType, ValType};

    fn create_i32_add_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0F],
        });
        module
    }

    fn create_i32_const_module() -> Module {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x2A, 0x0F],
        });
        module
    }

    fn create_i64_add_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I64), ValType::Num(NumType::I64)],
            vec![ValType::Num(NumType::I64)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x7C, 0x0F],
        });
        module
    }

    fn create_f32_add_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::F32), ValType::Num(NumType::F32)],
            vec![ValType::Num(NumType::F32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x8C, 0x0F],
        });
        module
    }

    fn create_f64_add_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::F64), ValType::Num(NumType::F64)],
            vec![ValType::Num(NumType::F64)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x92, 0x0F],
        });
        module
    }

    #[test]
    fn test_llvm_jit_creation() {
        let result = LlvmJit::new("test_module");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_i32_add() {
        let mut jit = LlvmJit::new("test_i32_add").unwrap();
        let module = create_i32_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.len(), 1);
        assert!(!compiled[0].entry_point.is_null());
    }

    #[test]
    fn test_invoke_i32_const() {
        let mut jit = LlvmJit::new("test_i32_const").unwrap();
        let module = create_i32_const_module();
        jit.compile_module(&module).unwrap();

        let result = jit.invoke_function(0, &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], WasmValue::I32(42));
    }

    #[test]
    fn test_compile_i64_add() {
        let mut jit = LlvmJit::new("test_i64_add").unwrap();
        let module = create_i64_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_f32_add() {
        let mut jit = LlvmJit::new("test_f32_add").unwrap();
        let module = create_f32_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_f64_add() {
        let mut jit = LlvmJit::new("test_f64_add").unwrap();
        let module = create_f64_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_function_entry() {
        let mut jit = LlvmJit::new("test_get_entry").unwrap();
        let module = create_i32_add_module();
        jit.compile_module(&module).unwrap();

        let entry = jit.get_function_entry(0);
        assert!(entry.is_some());
        assert!(!entry.unwrap().is_null());
    }

    #[test]
    fn test_get_nonexistent_function() {
        let jit = LlvmJit::new("test_nonexistent").unwrap();
        let entry = jit.get_function_entry(999);
        assert!(entry.is_none());
    }

    #[test]
    fn test_register_host_function() {
        extern "C" fn test_host_func() -> i32 {
            42
        }

        let mut jit = LlvmJit::new("test_host").unwrap();
        let result = jit.register_host_function("test_func", test_host_func as *const u8);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invoke_nonexistent_function() {
        let jit = LlvmJit::new("test_invoke_nonexistent").unwrap();
        let result = jit.invoke_function(999, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_functions() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x01, 0x0F],
        });
        module.funcs.push(Func {
            type_idx: 1,
            locals: vec![],
            body: vec![0x20, 0x00, 0x41, 0x01, 0x6A, 0x0F],
        });

        let mut jit = LlvmJit::new("test_multi").unwrap();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.len(), 2);
    }
}
