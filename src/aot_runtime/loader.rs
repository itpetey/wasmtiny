use crate::loader::Parser;
use crate::runtime::{
    Global, GlobalType, Memory, Module, Result, Table, ValType, WasmError, WasmValue,
};

use super::runtime::{AotExport, AotModule};

pub struct AotLoader {
    parser: Parser,
}

impl AotLoader {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    pub fn load(&self, data: &[u8]) -> Result<AotModule> {
        self.validate(data)?;
        let module = self.parser.parse(data)?;
        self.convert_to_aot_module(&module)
    }

    pub fn load_wasm(&self, data: &[u8]) -> Result<AotModule> {
        self.load(data)
    }

    fn validate(&self, data: &[u8]) -> Result<()> {
        if data.len() < 4 {
            return Err(WasmError::Load("AOT data too short".to_string()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x6D736100 {
            return Err(WasmError::Load("Invalid AOT magic".to_string()));
        }

        Ok(())
    }

    fn convert_to_aot_module(&self, module: &Module) -> Result<AotModule> {
        let mut aot_module = AotModule::from_module(module);

        for mem_type in &module.memories {
            let memory = Memory::new(mem_type.clone());
            aot_module.set_memory(memory);
        }

        for table_type in &module.tables {
            let table = Table::new(table_type.clone());
            aot_module.add_table(table);
        }

        for global_type in &module.globals {
            let global = Global::new(global_type.clone(), WasmValue::I32(0)).unwrap_or_else(|_| {
                Global::new(
                    GlobalType::new(ValType::Num(crate::runtime::NumType::I32), false),
                    WasmValue::I32(0),
                )
                .unwrap()
            });
            aot_module.add_global(global);
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

impl Default for AotLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::NumType;

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
}
