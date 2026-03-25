#![allow(unsafe_op_in_unsafe_fn)]

use crate::runtime::{
    FunctionType, JitState, Module, NumType, Result, SuspendedHandle, SuspensionKind,
    SuspensionState, ValType, WasmError, WasmValue,
};
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::ThreadId;

static NEXT_JIT_ID: AtomicU64 = AtomicU64::new(1);
static LLVM_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

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
use super::llvm_runtime::{
    clear_trap, configure_safepoints, current_execution_context_id, has_execution_context,
    take_runtime_error, take_suspended_handle, take_trap,
};
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
    #[allow(dead_code)]
    thread_safe_context: LLVMOrcThreadSafeContextRef,
    #[cfg(feature = "llvm-jit")]
    lljit: LLVMOrcLLJITRef,
    #[cfg(feature = "llvm-jit")]
    main_dylib: LLVMOrcJITDylibRef,
    compiled_functions: HashMap<u32, CompiledFunction>,
    #[cfg(feature = "llvm-jit")]
    helpers_registered: bool,
    safepoints_enabled: bool,
    suspend_requested: bool,
    suspended_handle: Option<SuspendedHandle>,
    active_suspension_id: Option<u64>,
    resumed_state: Option<JitState>,
    jit_id: u64,
    execution_epoch: u64,
    last_execution_thread: Option<ThreadId>,
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
    fn initialise_llvm() -> Result<()> {
        LLVM_INIT_RESULT
            .get_or_init(|| unsafe {
                if LLVM_InitializeNativeTarget() != 0 {
                    return Err("Failed to initialize native target".to_string());
                }
                if LLVM_InitializeNativeAsmPrinter() != 0 {
                    return Err("Failed to initialize native ASM printer".to_string());
                }
                if LLVM_InitializeNativeAsmParser() != 0 {
                    return Err("Failed to initialize native ASM parser".to_string());
                }
                Ok(())
            })
            .clone()
            .map_err(WasmError::Runtime)
    }

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
            Self::initialise_llvm()?;

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
                safepoints_enabled: false,
                suspend_requested: false,
                suspended_handle: None,
                active_suspension_id: None,
                resumed_state: None,
                jit_id: NEXT_JIT_ID.fetch_add(1, Ordering::SeqCst),
                execution_epoch: 0,
                last_execution_thread: None,
            })
        }
    }

    #[cfg(not(feature = "llvm-jit"))]
    pub fn new(_module_name: &str) -> Result<Self> {
        Err(WasmError::Runtime(
            "LLVM JIT not available: compile with --features llvm-jit".to_string(),
        ))
    }

    pub fn enable_safepoints(&mut self) {
        self.safepoints_enabled = true;
    }

    pub fn disable_safepoints(&mut self) {
        self.safepoints_enabled = false;
        self.suspend_requested = false;
    }

    pub fn request_safepoint(&mut self) -> Result<()> {
        if !self.safepoints_enabled {
            return Err(WasmError::Runtime(
                "JIT safepoints are not enabled".to_string(),
            ));
        }
        if self.active_suspension_id.is_some() || self.resumed_state.is_some() {
            return Err(WasmError::Runtime(
                "cannot request safepoint while suspended state is pending".to_string(),
            ));
        }
        self.suspend_requested = true;
        Ok(())
    }

    pub fn take_suspended_handle(&mut self) -> Option<SuspendedHandle> {
        self.suspended_handle.take()
    }

    pub fn is_suspended(&self) -> bool {
        self.active_suspension_id.is_some() || self.resumed_state.is_some()
    }

    pub fn try_resume(&mut self, handle: &SuspendedHandle) -> Result<()> {
        if let Some(thread_id) = self.last_execution_thread
            && thread_id != std::thread::current().id()
        {
            return Err(WasmError::Runtime(
                "cross-thread JIT resume is unsupported".to_string(),
            ));
        }

        if let Some(handle_jit_id) = handle.jit_id()
            && handle_jit_id != self.jit_id
        {
            return Err(WasmError::Runtime(
                "suspended handle is from a different JIT instance".to_string(),
            ));
        }

        if let Some(handle_epoch) = handle.jit_execution_epoch()
            && handle_epoch != self.execution_epoch
        {
            return Err(WasmError::Runtime(
                "suspended handle is from a previous JIT execution epoch".to_string(),
            ));
        }

        if let Some(active_suspension_id) = self.active_suspension_id
            && handle.instance_id() != active_suspension_id
        {
            return Err(WasmError::Runtime(
                "suspended handle does not match the active JIT suspension".to_string(),
            ));
        }

        let state = handle
            .resume()
            .map_err(|e| WasmError::Runtime(format!("resume failed: {}", e)))?;
        match state {
            SuspensionState::Jit(jit_state) => {
                self.suspended_handle = None;
                self.active_suspension_id = None;
                self.resumed_state = Some(jit_state);
                Ok(())
            }
            _ => Err(WasmError::Runtime("invalid JIT resume state".to_string())),
        }
    }

    pub fn continue_execution(&mut self) -> Result<Vec<WasmValue>> {
        if self.last_execution_thread != Some(std::thread::current().id()) {
            return Err(WasmError::Runtime(
                "cross-thread JIT continue is unsupported".to_string(),
            ));
        }

        if !has_execution_context() {
            return Err(WasmError::Runtime(
                "JIT continue requires a current execution context".to_string(),
            ));
        }

        if let Some(JitState::Pending { context_id, .. }) = self.resumed_state.as_ref()
            && current_execution_context_id() != Some(*context_id)
        {
            return Err(WasmError::Runtime(
                "JIT continue requires the original execution context".to_string(),
            ));
        }

        let state = self
            .resumed_state
            .take()
            .ok_or_else(|| WasmError::Runtime("no resumed JIT state to continue".to_string()))?;

        match state {
            JitState::Pending {
                func_idx,
                args,
                jit_id: _,
                execution_epoch: _,
                context_id: _,
                resume_pc: 0,
            } => self.invoke_function_internal(func_idx, &args, false),
            JitState::Pending { resume_pc, .. } => Err(WasmError::Runtime(format!(
                "unsupported JIT resume pc {}",
                resume_pc
            ))),
            JitState::Suspended { .. } => Err(WasmError::Runtime(
                "unsupported JIT suspended register state".to_string(),
            )),
        }
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
        register_helper!(
            "llvm_jit_safepoint_entry",
            llvm_jit_safepoint_entry as *const u8
        );
        register_helper!("llvm_jit_call_import", llvm_jit_call_import as *const u8);
        register_helper!("llvm_jit_meter_tick", llvm_jit_meter_tick as *const u8);
        register_helper!("llvm_jit_memory_size", llvm_jit_memory_size as *const u8);
        register_helper!("llvm_jit_memory_grow", llvm_jit_memory_grow as *const u8);

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
    /// The same requirement applies before calling `continue_execution()`.
    #[cfg(feature = "llvm-jit")]
    pub fn invoke_function(&mut self, func_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.invoke_function_internal(func_idx, args, true)
    }

    #[cfg(feature = "llvm-jit")]
    fn invoke_function_internal(
        &mut self,
        func_idx: u32,
        args: &[WasmValue],
        start_new_epoch: bool,
    ) -> Result<Vec<WasmValue>> {
        if self.resumed_state.is_some() {
            return Err(WasmError::Runtime(
                "cannot invoke JIT function: resumed state must be continued first".to_string(),
            ));
        }
        if self.active_suspension_id.is_some() {
            return Err(WasmError::Runtime(
                "cannot invoke JIT function: suspended handle must be resumed first".to_string(),
            ));
        }

        if start_new_epoch {
            self.execution_epoch += 1;
            self.last_execution_thread = Some(std::thread::current().id());
        } else {
            if self.last_execution_thread != Some(std::thread::current().id()) {
                return Err(WasmError::Runtime(
                    "cross-thread JIT continue is unsupported".to_string(),
                ));
            }
            if !has_execution_context() {
                return Err(WasmError::Runtime(
                    "JIT continue requires a current execution context".to_string(),
                ));
            }
        }

        let clear_requested = |jit: &mut Self| {
            jit.suspend_requested = false;
        };

        let compiled = match self.compiled_functions.get(&func_idx) {
            Some(compiled) => compiled,
            None => {
                clear_requested(self);
                return Err(WasmError::Runtime(format!(
                    "Function {} not compiled",
                    func_idx
                )));
            }
        };

        let entry_point = compiled.entry_point;
        let func_type = compiled.func_type.clone();

        if args.len() != func_type.params.len() {
            clear_requested(self);
            return Err(WasmError::Runtime(format!(
                "Argument count mismatch: expected {}, got {}",
                func_type.params.len(),
                args.len()
            )));
        }

        for (idx, (arg, expected_type)) in args.iter().zip(func_type.params.iter()).enumerate() {
            if arg.val_type() != *expected_type {
                clear_requested(self);
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
            match Self::pack_arg(arg) {
                Ok(packed) => packed_args.push(packed),
                Err(error) => {
                    clear_requested(self);
                    return Err(error);
                }
            }
        }
        let mut packed_results = vec![0u64; func_type.results.len().max(1)];

        unsafe {
            clear_trap();
            configure_safepoints(
                self.safepoints_enabled,
                self.suspend_requested,
                self.jit_id,
                self.execution_epoch,
            );
            let func: extern "C" fn(*const u64, *mut u64) = std::mem::transmute(entry_point);
            func(packed_args.as_ptr(), packed_results.as_mut_ptr());

            if let Some(handle) = take_suspended_handle() {
                self.active_suspension_id = Some(handle.instance_id());
                self.suspended_handle = Some(handle);
                self.suspend_requested = false;
                return Err(WasmError::Suspended(SuspensionKind::Safepoint));
            }

            if let Some(message) = take_runtime_error() {
                self.suspend_requested = false;
                return Err(WasmError::Runtime(message));
            }

            if let Some(code) = take_trap() {
                self.suspend_requested = false;
                return Err(WasmError::Trap(code));
            }
        }

        let mut results = Vec::with_capacity(func_type.results.len());
        for (idx, result_type) in func_type.results.iter().copied().enumerate() {
            match Self::unpack_result(packed_results[idx], result_type) {
                Ok(result) => results.push(result),
                Err(error) => {
                    self.suspend_requested = false;
                    return Err(error);
                }
            }
        }

        self.suspend_requested = false;
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
    use crate::aot_runtime::runtime::AotModule;
    use crate::jit::set_execution_context;
    use crate::runtime::{
        Func, FunctionType, HostCallOutcome, HostFunc, Import, ImportKind, InstanceLimits, Limits,
        Module, NumType, ValType, WasmValue,
    };
    use std::sync::{Mutex as StdMutex, OnceLock};

    fn llvm_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LLVM_TEST_LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LLVM_TEST_LOCK
            .get_or_init(|| StdMutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set_jit_context(module: &Module) -> Box<AotModule> {
        let mut aot_module = Box::new(AotModule::from_module(module));
        let memory_context = aot_module
            .memory_context()
            .unwrap_or((std::ptr::null_mut(), 0));
        set_execution_context(
            &mut *aot_module as *mut _,
            memory_context.0,
            memory_context.1,
        );
        aot_module
    }

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

    fn create_budget_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x01, 0x1A, 0x41, 0x02, 0x1A, 0x0F],
        });
        module
    }

    fn create_memory_grow_module() -> Module {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module
            .memories
            .push(crate::runtime::MemoryType::new(Limits::Min(1)));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x01, 0x40, 0x00, 0x0F],
        });
        module
    }

    fn create_memory_grow_fail_module() -> Module {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module
            .memories
            .push(crate::runtime::MemoryType::new(Limits::Min(1)));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x7F, 0x40, 0x00, 0x0F],
        });
        module
    }

    #[test]
    fn test_jit_stats_are_monotonic() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_jit_stats_are_monotonic").unwrap();
        let module = create_i32_const_module();
        jit.compile_module(&module).unwrap();

        let aot_module = set_jit_context(&module);
        let first = jit.invoke_function(0, &[]).unwrap();
        assert_eq!(first, vec![WasmValue::I32(42)]);
        let first_stats = aot_module.instance_stats().unwrap();

        let second = jit.invoke_function(0, &[]).unwrap();
        assert_eq!(second, vec![WasmValue::I32(42)]);
        let second_stats = aot_module.instance_stats().unwrap();

        assert!(second_stats.executed_instructions > first_stats.executed_instructions);
    }

    #[test]
    fn test_jit_execution_budget_is_enforced() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_jit_execution_budget_is_enforced").unwrap();
        let module = create_budget_module();
        jit.compile_module(&module).unwrap();

        let mut aot_module = set_jit_context(&module);
        aot_module
            .set_instance_limits(InstanceLimits::new(Some(2), None))
            .unwrap();

        let error = jit.invoke_function(0, &[]).unwrap_err();
        assert_eq!(
            error,
            WasmError::Trap(crate::runtime::TrapCode::ExecutionBudgetExceeded)
        );
    }

    #[test]
    fn test_jit_memory_limit_is_enforced() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_jit_memory_limit_is_enforced").unwrap();
        let module = create_memory_grow_module();
        jit.compile_module(&module).unwrap();

        let mut aot_module = set_jit_context(&module);
        aot_module
            .set_instance_limits(InstanceLimits::new(None, Some(1)))
            .unwrap();

        let error = jit.invoke_function(0, &[]).unwrap_err();
        assert_eq!(
            error,
            WasmError::Trap(crate::runtime::TrapCode::MemoryLimitExceeded)
        );
        assert_eq!(aot_module.instance_stats().unwrap().memory_pages, 1);
    }

    #[test]
    fn test_jit_memory_grow_with_negative_delta_returns_minus_one() {
        let _guard = llvm_test_guard();
        let mut jit =
            LlvmJit::new("test_jit_memory_grow_with_negative_delta_returns_minus_one").unwrap();
        let module = create_memory_grow_fail_module();
        jit.compile_module(&module).unwrap();

        let aot_module = set_jit_context(&module);
        let result = jit.invoke_function(0, &[]).unwrap();

        assert_eq!(result, vec![WasmValue::I32(-1)]);
        assert_eq!(aot_module.instance_stats().unwrap().memory_pages, 1);
    }

    #[test]
    fn test_jit_meter_overflow_surfaces_runtime_error() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_jit_meter_overflow_surfaces_runtime_error").unwrap();
        let module = create_i32_const_module();
        jit.compile_module(&module).unwrap();

        let aot_module = set_jit_context(&module);
        aot_module.record_execution(u64::MAX).unwrap();

        let error = jit.invoke_function(0, &[]).unwrap_err();
        assert!(matches!(error, WasmError::Runtime(message) if message.contains("overflowed")));
    }

    #[test]
    fn test_llvm_jit_creation() {
        let _guard = llvm_test_guard();
        let result = LlvmJit::new("test_module");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_i32_add() {
        let _guard = llvm_test_guard();
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
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_i32_const").unwrap();
        let module = create_i32_const_module();
        jit.compile_module(&module).unwrap();
        let _aot_module = set_jit_context(&module);

        let result = jit.invoke_function(0, &[]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], WasmValue::I32(42));
    }

    #[test]
    fn test_compile_i64_add() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_i64_add").unwrap();
        let module = create_i64_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_f32_add() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_f32_add").unwrap();
        let module = create_f32_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_f64_add() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_f64_add").unwrap();
        let module = create_f64_add_module();
        let result = jit.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_function_entry() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_get_entry").unwrap();
        let module = create_i32_add_module();
        jit.compile_module(&module).unwrap();

        let entry = jit.get_function_entry(0);
        assert!(entry.is_some());
        assert!(!entry.unwrap().is_null());
    }

    #[test]
    fn test_get_nonexistent_function() {
        let _guard = llvm_test_guard();
        let jit = LlvmJit::new("test_nonexistent").unwrap();
        let entry = jit.get_function_entry(999);
        assert!(entry.is_none());
    }

    #[test]
    fn test_register_host_function() {
        let _guard = llvm_test_guard();
        extern "C" fn test_host_func() -> i32 {
            42
        }

        let mut jit = LlvmJit::new("test_host").unwrap();
        let result = jit.register_host_function("test_func", test_host_func as *const u8);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invoke_nonexistent_function() {
        let _guard = llvm_test_guard();
        let mut jit = LlvmJit::new("test_invoke_nonexistent").unwrap();
        let result = jit.invoke_function(999, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_entry_safepoint_suspend_and_resume() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_entry_safepoint").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let _aot_module = set_jit_context(&module);

        let result = jit.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));
        assert!(jit.is_suspended());

        let handle = jit
            .take_suspended_handle()
            .expect("expected suspended handle");
        jit.try_resume(&handle).unwrap();
        let resumed = jit.continue_execution().unwrap();
        assert_eq!(resumed, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_continue_execution_requires_fresh_runtime_context() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_require_context").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let mut aot_module = set_jit_context(&module);

        let result = jit.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let handle = jit.take_suspended_handle().unwrap();
        jit.try_resume(&handle).unwrap();

        set_execution_context(std::ptr::null_mut(), std::ptr::null_mut(), 0);

        let failed = jit.continue_execution();
        assert!(matches!(
            failed,
            Err(WasmError::Runtime(msg)) if msg.contains("current execution context")
        ));

        let memory_context = aot_module
            .memory_context()
            .unwrap_or((std::ptr::null_mut(), 0));
        set_execution_context(
            &mut *aot_module as *mut _,
            memory_context.0,
            memory_context.1,
        );
        let resumed = jit.continue_execution().unwrap();
        assert_eq!(resumed, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_continue_execution_rejects_wrong_runtime_context() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_wrong_context").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let mut original_aot_module = set_jit_context(&module);
        let result = jit.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let handle = jit.take_suspended_handle().unwrap();
        jit.try_resume(&handle).unwrap();

        let _wrong_aot_module = set_jit_context(&module);
        let failed = jit.continue_execution();
        assert!(matches!(
            failed,
            Err(WasmError::Runtime(msg)) if msg.contains("original execution context")
        ));

        let memory_context = original_aot_module
            .memory_context()
            .unwrap_or((std::ptr::null_mut(), 0));
        set_execution_context(
            &mut *original_aot_module as *mut _,
            memory_context.0,
            memory_context.1,
        );
        let resumed = jit.continue_execution().unwrap();
        assert_eq!(resumed, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_wrong_jit_resume_fails() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit1 = LlvmJit::new("test_wrong_jit_1").unwrap();
        let mut jit2 = LlvmJit::new("test_wrong_jit_2").unwrap();
        jit1.compile_module(&module).unwrap();
        jit2.compile_module(&module).unwrap();
        jit1.enable_safepoints();
        jit1.request_safepoint().unwrap();

        let _aot_module = set_jit_context(&module);
        let result = jit1.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let handle = jit1
            .take_suspended_handle()
            .expect("expected suspended handle");
        let resume_result = jit2.try_resume(&handle);
        assert!(matches!(
            resume_result,
            Err(WasmError::Runtime(msg)) if msg.contains("different JIT instance")
        ));
    }

    #[test]
    fn test_stale_jit_handle_resume_fails() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_stale_jit").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();

        jit.request_safepoint().unwrap();
        let _first_aot_module = set_jit_context(&module);
        let first = jit.invoke_function(0, &[]);
        assert!(matches!(
            first,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));
        let stale_handle = jit.take_suspended_handle().expect("expected first handle");

        jit.try_resume(&stale_handle).unwrap();
        let resumed = jit.continue_execution().unwrap();
        assert_eq!(resumed, vec![WasmValue::I32(42)]);

        jit.request_safepoint().unwrap();
        let _second_aot_module = set_jit_context(&module);
        let second = jit.invoke_function(0, &[]);
        assert!(matches!(
            second,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let resume_result = jit.try_resume(&stale_handle);
        assert!(matches!(
            resume_result,
            Err(WasmError::Runtime(msg)) if msg.contains("previous JIT execution epoch")
        ));
    }

    #[test]
    fn test_pending_hostcall_import_is_rejected() {
        let _guard = llvm_test_guard();
        struct PendingHost;

        impl HostFunc for PendingHost {
            fn call(
                &self,
                _store: &mut crate::runtime::Store,
                _args: &[WasmValue],
            ) -> Result<Vec<WasmValue>> {
                panic!("pending hostcall should not complete synchronously")
            }

            fn call_with_suspension(
                &self,
                _store: &mut crate::runtime::Store,
                _args: &[WasmValue],
            ) -> Result<HostCallOutcome> {
                Ok(HostCallOutcome::Pending {
                    pending_work: vec![1, 2, 3],
                })
            }

            fn function_type(&self) -> Option<&FunctionType> {
                static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
                Some(FUNC_TYPE.get_or_init(|| FunctionType::new(vec![], vec![])))
            }
        }

        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });
        module.funcs.push(Func {
            type_idx: 1,
            locals: vec![],
            body: vec![0x10, 0x00, 0x41, 0x2A, 0x0F],
        });

        let mut jit = LlvmJit::new("test_pending_hostcall_import").unwrap();
        jit.compile_module(&module).unwrap();

        let mut aot_module = AotModule::from_module(&module);
        aot_module
            .register_host_import(
                "env",
                "host",
                Box::new(PendingHost),
                FunctionType::new(vec![], vec![]),
            )
            .unwrap();
        let memory_context = aot_module
            .memory_context()
            .unwrap_or((std::ptr::null_mut(), 0));
        set_execution_context(
            &mut aot_module as *mut _,
            memory_context.0,
            memory_context.1,
        );

        let result = jit.invoke_function(1, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(msg)) if msg.contains("unsupported in JIT import path")
        ));
    }

    #[test]
    fn test_failed_invocation_clears_pending_safepoint_request() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_clear_stale_request").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let failed = jit.invoke_function(999, &[]);
        assert!(matches!(
            failed,
            Err(WasmError::Runtime(msg)) if msg.contains("not compiled")
        ));

        let _aot_module = set_jit_context(&module);
        let result = jit.invoke_function(0, &[]).unwrap();
        assert_eq!(result, vec![WasmValue::I32(42)]);
        assert!(!jit.is_suspended());
    }

    #[test]
    fn test_cross_thread_continue_is_rejected() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_cross_thread_continue").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let _aot_module = set_jit_context(&module);
        let result = jit.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let handle = jit.take_suspended_handle().unwrap();
        jit.try_resume(&handle).unwrap();

        let other_thread_id = std::thread::spawn(|| std::thread::current().id())
            .join()
            .unwrap();
        jit.last_execution_thread = Some(other_thread_id);

        let result = jit.continue_execution();
        assert!(matches!(
            result,
            Err(WasmError::Runtime(msg)) if msg.contains("cross-thread JIT continue")
        ));
    }

    #[test]
    fn test_take_suspended_handle_keeps_jit_blocked() {
        let _guard = llvm_test_guard();
        let module = create_i32_const_module();
        let mut jit = LlvmJit::new("test_take_handle_blocks_jit").unwrap();
        jit.compile_module(&module).unwrap();
        jit.enable_safepoints();
        jit.request_safepoint().unwrap();

        let _aot_module = set_jit_context(&module);
        let result = jit.invoke_function(0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let _handle = jit.take_suspended_handle().unwrap();
        assert!(jit.is_suspended());

        let request = jit.request_safepoint();
        assert!(matches!(
            request,
            Err(WasmError::Runtime(msg)) if msg.contains("suspended state is pending")
        ));
    }

    #[test]
    fn test_multiple_functions() {
        let _guard = llvm_test_guard();
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
