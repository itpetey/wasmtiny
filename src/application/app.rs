use crate::aot_runtime::loader::AotLoader;
use crate::aot_runtime::runtime::{AotExport, AotModule, AotRuntime};
use crate::runtime::{Result, Store, WasmError, WasmValue};
use std::fs;
use std::path::Path;

pub struct WasmApplication {
    runtime: AotRuntime,
    loader: AotLoader,
}

impl WasmApplication {
    pub fn new() -> Self {
        Self {
            runtime: AotRuntime::new(),
            loader: AotLoader::new(),
        }
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

        Ok(())
    }

    pub fn call_function(
        &self,
        module_idx: u32,
        func_name: &str,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(AotExport::Function(func_idx)) = module.get_export(func_name) {
            module.call_native(*func_idx, args)
        } else {
            Err(WasmError::Runtime(format!(
                "function {} not found",
                func_name
            )))
        }
    }

    pub fn execute_main(&self, module_idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        self.call_function(module_idx, "main", args)
    }

    pub fn execute_start(&self, module_idx: u32) -> Result<()> {
        let module = self
            .runtime
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        let store = Store::new();

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
}
