use crate::aot_runtime::loader::AotLoader;
use crate::aot_runtime::runtime::{AotExport, AotRuntime};
use crate::runtime::{FunctionType, HostFunc, Result, WasmError, WasmValue};
use std::fs;
use std::path::Path;

#[cfg(feature = "llvm-jit")]
use crate::jit::LlvmJit;

pub enum ExecutionMode {
    Interpreter,
    #[cfg(feature = "llvm-jit")]
    LlvmJit,
}

pub struct WasmApplication {
    runtime: AotRuntime,
    loader: AotLoader,
    #[cfg(feature = "llvm-jit")]
    llvm_jit: Option<LlvmJit>,
    #[cfg(feature = "llvm-jit")]
    execution_mode: ExecutionMode,
}

impl WasmApplication {
    pub fn new() -> Self {
        Self {
            runtime: AotRuntime::new(),
            loader: AotLoader::new(),
            #[cfg(feature = "llvm-jit")]
            llvm_jit: None,
            #[cfg(feature = "llvm-jit")]
            execution_mode: ExecutionMode::Interpreter,
        }
    }

    #[cfg(feature = "llvm-jit")]
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.execution_mode = mode;
    }

    #[cfg(feature = "llvm-jit")]
    pub fn execution_mode(&self) -> &ExecutionMode {
        &self.execution_mode
    }

    #[cfg(feature = "llvm-jit")]
    pub fn compile_with_llvm(&mut self, module_idx: u32) -> Result<()> {
        let aot_module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        let module_name = format!("module_{}", module_idx);
        let mut llvm_jit = LlvmJit::new(&module_name)?;

        match llvm_jit.compile_module(aot_module.module()) {
            Ok(_) => {
                self.llvm_jit = Some(llvm_jit);
                self.execution_mode = ExecutionMode::LlvmJit;
                Ok(())
            }
            Err(e) => {
                self.execution_mode = ExecutionMode::Interpreter;
                Err(e)
            }
        }
    }

    #[cfg(feature = "llvm-jit")]
    pub fn compile_with_llvm_fallback(&mut self, module_idx: u32) {
        let _ = self.compile_with_llvm(module_idx);
    }

    pub fn load_module_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<u32> {
        let data = fs::read(path)?;
        self.load_module_from_memory(&data)
    }

    pub fn load_module_from_memory(&mut self, data: &[u8]) -> Result<u32> {
        let aot_module = self.loader.load(data)?;
        let module_idx = self.runtime.modules.len() as u32;
        self.runtime.modules.push(aot_module);
        Ok(module_idx)
    }

    pub fn instantiate(&mut self, module_idx: u32) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.instantiate()
    }

    pub fn register_host_function(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        func: Box<dyn HostFunc>,
        func_type: FunctionType,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_host_import(import_module, name, func, func_type)
    }

    pub fn register_memory_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        memory: crate::runtime::Memory,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_memory_import(import_module, name, memory)
    }

    pub fn register_table_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        table: crate::runtime::Table,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_table_import(import_module, name, table)
    }

    pub fn register_global_import(
        &mut self,
        module_idx: u32,
        import_module: &str,
        name: &str,
        global: crate::runtime::Global,
    ) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Instantiate(format!("module {} not found", module_idx)))?;
        module.register_global_import(import_module, name, global)
    }

    pub fn call_function(
        &mut self,
        module_idx: u32,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        #[cfg(feature = "llvm-jit")]
        if matches!(self.execution_mode, ExecutionMode::LlvmJit)
            && let Some(ref llvm_jit) = self.llvm_jit
        {
            let module = self
                .runtime
                .get_module(module_idx)
                .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

            if let Some(AotExport::Function(func_idx)) = module.get_export(func_name).cloned() {
                let adjusted_idx = func_idx
                    - module
                        .imports()
                        .iter()
                        .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
                        .count() as u32;
                return llvm_jit.invoke_function(adjusted_idx, args);
            } else {
                return Err(WasmError::Runtime(format!(
                    "function {} not found",
                    func_name
                )));
            }
        }

        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(AotExport::Function(func_idx)) = module.get_export(func_name).cloned() {
            module.invoke_function(func_idx, args)
        } else {
            Err(WasmError::Runtime(format!(
                "function {} not found",
                func_name
            )))
        }
    }

    pub fn execute_main(&mut self, module_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.call_function(module_idx, "main", args)
    }

    pub fn execute_start(&mut self, module_idx: u32) -> Result<()> {
        let module = self
            .runtime
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        if let Some(start_idx) = module.start_function() {
            let _ = module.invoke_function(start_idx, &[])?;
        }

        Ok(())
    }
}

impl Default for WasmApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{FunctionType, Global, GlobalType, NumType, ValType};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct CountingHostFunc;

    impl HostFunc for CountingHostFunc {
        fn call(
            &self,
            store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            let count = store.get_native_func_count() as i32;
            store.register_native(Box::new(NoopHostFunc), FunctionType::empty());
            Ok(vec![WasmValue::I32(count)])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(
                FUNC_TYPE
                    .get_or_init(|| FunctionType::new(vec![], vec![ValType::Num(NumType::I32)])),
            )
        }
    }

    struct NoopHostFunc;

    impl HostFunc for NoopHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            Ok(vec![])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(FUNC_TYPE.get_or_init(FunctionType::empty))
        }
    }

    struct StartHostFunc {
        calls: Arc<AtomicUsize>,
    }

    impl HostFunc for StartHostFunc {
        fn call(
            &self,
            _store: &mut crate::runtime::Store,
            _args: &[WasmValue],
        ) -> Result<Vec<WasmValue>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        fn function_type(&self) -> Option<&FunctionType> {
            static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
            Some(FUNC_TYPE.get_or_init(FunctionType::empty))
        }
    }

    #[test]
    fn test_application_creation() {
        let app = WasmApplication::new();
        assert_eq!(app.runtime.modules.len(), 0);
    }

    #[test]
    fn test_load_module_from_memory() {
        let mut app = WasmApplication::new();

        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let result = app.load_module_from_memory(&wasm_data);
        assert!(result.is_ok());

        let idx = result.unwrap();
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_instantiate() {
        let mut app = WasmApplication::new();

        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        let result = app.instantiate(idx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_instantiate_rejects_missing_imports() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00, 0x00,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        assert!(app.instantiate(idx).is_err());
    }

    #[test]
    fn test_call_function_with_host_import() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00,
            0x00, 0x03, 0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00,
            0x01, 0x0A, 0x06, 0x01, 0x04, 0x00, 0x10, 0x00, 0x0B,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "host",
            Box::new(CountingHostFunc),
            FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
        )
        .unwrap();
        let first = app.call_function(idx, "main", &[]).unwrap();
        let second = app.call_function(idx, "main", &[]).unwrap();

        assert_eq!(first, vec![WasmValue::I32(0)]);
        assert_eq!(second, vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_call_exported_imported_function() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'h', b'o', b's', b't', 0x00,
            0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "host",
            Box::new(CountingHostFunc),
            FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]),
        )
        .unwrap();

        let first = app.call_function(idx, "main", &[]).unwrap();
        let second = app.call_function(idx, "main", &[]).unwrap();

        assert_eq!(first, vec![WasmValue::I32(0)]);
        assert_eq!(second, vec![WasmValue::I32(1)]);
    }

    #[test]
    fn test_call_function_with_global_import() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7F, 0x02, 0x0A, 0x01, 0x03, b'e', b'n', b'v', 0x01, b'g', 0x03, 0x7F, 0x01, 0x03,
            0x02, 0x01, 0x00, 0x07, 0x08, 0x01, 0x04, b'm', b'a', b'i', b'n', 0x00, 0x00, 0x0A,
            0x0A, 0x01, 0x08, 0x00, 0x41, 0x07, 0x24, 0x00, 0x23, 0x00, 0x0B,
        ];

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_global_import(
            idx,
            "env",
            "g",
            Global::new(
                GlobalType::new(ValType::Num(NumType::I32), true),
                WasmValue::I32(0),
            )
            .unwrap(),
        )
        .unwrap();

        let results = app.call_function(idx, "main", &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_execute_start_with_imported_function() {
        let mut app = WasmApplication::new();
        let wasm_data = vec![
            0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
            0x02, 0x0C, 0x01, 0x03, b'e', b'n', b'v', 0x04, b'i', b'n', b'i', b't', 0x00, 0x00,
            0x08, 0x01, 0x00,
        ];
        let calls = Arc::new(AtomicUsize::new(0));

        let idx = app.load_module_from_memory(&wasm_data).unwrap();
        app.register_host_function(
            idx,
            "env",
            "init",
            Box::new(StartHostFunc {
                calls: calls.clone(),
            }),
            FunctionType::empty(),
        )
        .unwrap();

        app.execute_start(idx).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
