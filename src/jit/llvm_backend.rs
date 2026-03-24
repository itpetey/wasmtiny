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

use super::wasm_to_llvm::WasmToLlvmTranslator;

struct CompiledFunction {
    entry_point: *const u8,
    func_type: FunctionType,
}

pub struct LlvmJit {
    #[cfg(feature = "llvm-jit")]
    thread_safe_context: LLVMOrcThreadSafeContextRef,
    #[cfg(feature = "llvm-jit")]
    lljit: LLVMOrcLLJITRef,
    #[cfg(feature = "llvm-jit")]
    main_dylib: LLVMOrcJITDylibRef,
    compiled_functions: HashMap<u32, CompiledFunction>,
}

#[derive(Debug, Clone)]
pub struct CompiledLlvmFunction {
    pub func_idx: u32,
    pub entry_point: *const u8,
}

impl LlvmJit {
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
    pub fn compile_module(&mut self, module: &Module) -> Result<Vec<CompiledLlvmFunction>> {
        unsafe {
            let mut compiled = Vec::new();

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

                    let (llvm_module, _llvm_func) = match translate_result {
                        Ok(result) => result,
                        Err(e) => {
                            LLVMOrcDisposeThreadSafeContext(ts_context);
                            return Err(e);
                        }
                    };

                    let compile_result = self.compile_and_add(llvm_module, ts_context, func_idx);

                    let entry_point = match compile_result {
                        Ok(ep) => ep,
                        Err(e) => {
                            return Err(e);
                        }
                    };

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
            }

            Ok(compiled)
        }
    }

    #[cfg(feature = "llvm-jit")]
    fn compile_and_add(
        &mut self,
        llvm_module: LLVMModuleRef,
        ts_context: LLVMOrcThreadSafeContextRef,
        func_idx: u32,
    ) -> Result<*const u8> {
        unsafe {
            let func_name = format!("wasm_func_{}", func_idx);
            let func_name_c = CString::new(func_name.clone())
                .map_err(|_| WasmError::Runtime("Function name contains NUL byte".to_string()))?;

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
            let mut symbol: LLVMOrcExecutorAddress = 0;
            let result = LLVMOrcLLJITLookup(self.lljit, &mut symbol, func_name_c.as_ptr());

            if !result.is_null() || symbol == 0 {
                return Err(WasmError::Runtime(format!(
                    "Failed to lookup symbol {}",
                    func_name
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

    #[cfg(feature = "llvm-jit")]
    pub fn invoke_function(&self, func_idx: u32, _args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let compiled = self
            .compiled_functions
            .get(&func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("Function {} not compiled", func_idx)))?;

        let entry_point = compiled.entry_point;
        let func_type = &compiled.func_type;

        unsafe {
            match func_type.results.as_slice() {
                [] => {
                    let func: extern "C" fn() = std::mem::transmute(entry_point);
                    func();
                    Ok(vec![])
                }
                [ValType::Num(NumType::I32)] => {
                    let func: extern "C" fn() -> i32 = std::mem::transmute(entry_point);
                    let result = func();
                    Ok(vec![WasmValue::I32(result)])
                }
                [ValType::Num(NumType::I64)] => {
                    let func: extern "C" fn() -> i64 = std::mem::transmute(entry_point);
                    let result = func();
                    Ok(vec![WasmValue::I64(result)])
                }
                [ValType::Num(NumType::F32)] => {
                    let func: extern "C" fn() -> f32 = std::mem::transmute(entry_point);
                    let result = func();
                    Ok(vec![WasmValue::F32(result)])
                }
                [ValType::Num(NumType::F64)] => {
                    let func: extern "C" fn() -> f64 = std::mem::transmute(entry_point);
                    let result = func();
                    Ok(vec![WasmValue::F64(result)])
                }
                _ => Err(WasmError::Runtime(
                    "Multi-value returns not yet supported".to_string(),
                )),
            }
        }
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
