use crate::loader::{Parser, Validator};
use crate::runtime::{Instance, Module, Result, WasmError};
use std::sync::{Arc, Mutex};

use super::runtime::{AotExport, AotModule};

pub struct AotLoader {
    parser: Parser,
    validator: Validator,
    store: Arc<Mutex<crate::runtime::Store>>,
}

impl AotLoader {
    pub fn new() -> Self {
        Self::with_store(Arc::new(Mutex::new(crate::runtime::Store::new())))
    }

    pub fn with_store(store: Arc<Mutex<crate::runtime::Store>>) -> Self {
        Self {
            parser: Parser::new(),
            validator: Validator::new(),
            store,
        }
    }

    pub fn load(&self, data: &[u8]) -> Result<AotModule> {
        self.load_with_store(data, self.store.clone())
    }

    fn load_with_store(
        &self,
        data: &[u8],
        store: Arc<Mutex<crate::runtime::Store>>,
    ) -> Result<AotModule> {
        let module = self.parse_validated_module(data)?;
        self.convert_to_aot_module(&module, store)
    }

    pub fn load_wasm(&self, data: &[u8]) -> Result<AotModule> {
        self.load(data)
    }

    pub fn validate(&self, data: &[u8]) -> Result<()> {
        self.parse_validated_module(data).map(|_| ())
    }

    fn parse_validated_module(&self, data: &[u8]) -> Result<Module> {
        let module = self.parser.parse(data)?;
        self.validator.validate(&module)?;
        Ok(module)
    }

    fn convert_to_aot_module(
        &self,
        module: &Module,
        store: Arc<Mutex<crate::runtime::Store>>,
    ) -> Result<AotModule> {
        let mut aot_module = AotModule::from_module_with_store(module, store.clone())?;
        if module.imports.is_empty() {
            let instance = Instance::new_with_store(Arc::new(module.clone()), store)?;
            aot_module.memories = instance
                .memories
                .iter()
                .map(|memory| {
                    memory
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|memory| memory.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            aot_module.tables = instance
                .tables
                .iter()
                .map(|table| {
                    table
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|table| table.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            aot_module.globals = instance
                .globals
                .iter()
                .map(|global| {
                    global
                        .lock()
                        .map_err(poisoned_lock)
                        .map(|global| global.clone())
                })
                .collect::<Result<Vec<_>>>()?;
        }

        for export in &module.exports {
            let export_idx = match &export.kind {
                crate::runtime::ExportKind::Func(idx) => AotExport::Function(*idx),
                crate::runtime::ExportKind::Table(idx) => AotExport::Table(*idx),
                crate::runtime::ExportKind::Memory(idx) => AotExport::Memory(*idx),
                crate::runtime::ExportKind::Global(idx) => AotExport::Global(*idx),
            };
            aot_module.exports.insert(export.name.clone(), export_idx);
        }

        Ok(aot_module)
    }
}

fn poisoned_lock<T>(_: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> WasmError {
    WasmError::Runtime("instance lock poisoned".to_string())
}

impl Default for AotLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_valid_aot() {
        let loader = AotLoader::new();
        let mut data = vec![0x00, 0x61, 0x73, 0x6D];
        data.extend_from_slice(&[1, 0, 0, 0]);
        assert!(loader.validate(&data).is_ok());
    }

    #[test]
    fn test_load_wasm_module() {
        let loader = AotLoader::new();
        let wasm_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let result = loader.load(&wasm_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rejects_truncated_header_with_valid_magic() {
        let loader = AotLoader::new();
        let truncated = vec![0x00, 0x61, 0x73, 0x6D];
        assert!(loader.validate(&truncated).is_err());
    }
}
