use crate::runtime::{Global, Memory, Module, Result, Table, WasmError, WasmValue};
use std::collections::HashMap;

pub struct AotModule {
    pub native_functions: Vec<NativeFunc>,
    pub memory: Option<Memory>,
    pub tables: Vec<Table>,
    pub globals: Vec<Global>,
    pub exports: HashMap<String, AotExport>,
}

impl std::fmt::Debug for AotModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AotModule")
            .field("native_functions", &self.native_functions.len())
            .field("memory", &self.memory.is_some())
            .field("tables", &self.tables)
            .field("globals", &self.globals)
            .field("exports", &self.exports)
            .finish()
    }
}

pub type NativeFunc = Box<dyn Fn(&[WasmValue]) -> Result<Vec<WasmValue>> + Send + Sync>;

#[derive(Debug, Clone)]
pub enum AotExport {
    Function(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

impl AotModule {
    pub fn from_module(_module: &Module) -> Self {
        Self {
            native_functions: Vec::new(),
            memory: None,
            tables: Vec::new(),
            globals: Vec::new(),
            exports: HashMap::new(),
        }
    }

    pub fn register_native(&mut self, func: NativeFunc) -> u32 {
        let idx = self.native_functions.len() as u32;
        self.native_functions.push(func);
        idx
    }

    pub fn call_native(&self, idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let func = self
            .native_functions
            .get(idx as usize)
            .ok_or_else(|| WasmError::Runtime(format!("native function {} not found", idx)))?;
        func(args)
    }

    pub fn get_export(&self, name: &str) -> Option<&AotExport> {
        self.exports.get(name)
    }

    pub fn set_memory(&mut self, memory: Memory) {
        self.memory = Some(memory);
    }

    pub fn get_memory(&self) -> Option<&Memory> {
        self.memory.as_ref()
    }

    pub fn add_table(&mut self, table: Table) -> u32 {
        let idx = self.tables.len() as u32;
        self.tables.push(table);
        idx
    }

    pub fn get_table(&self, idx: u32) -> Option<&Table> {
        self.tables.get(idx as usize)
    }

    pub fn add_global(&mut self, global: Global) -> u32 {
        let idx = self.globals.len() as u32;
        self.globals.push(global);
        idx
    }

    pub fn get_global(&self, idx: u32) -> Option<&Global> {
        self.globals.get(idx as usize)
    }

    pub fn get_global_mut(&mut self, idx: u32) -> Option<&mut Global> {
        self.globals.get_mut(idx as usize)
    }
}

pub struct AotRuntime {
    pub modules: Vec<AotModule>,
}

impl AotRuntime {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn load_module(&mut self, data: &[u8]) -> Result<u32> {
        if data.len() < 4 {
            return Err(WasmError::Load("AOT data too short".to_string()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x6D736100 {
            return Err(WasmError::Load("Invalid AOT magic".to_string()));
        }

        let module_idx = self.modules.len() as u32;
        self.modules.push(AotModule::from_module(&Module::new()));
        Ok(module_idx)
    }

    pub fn get_module(&self, idx: u32) -> Option<&AotModule> {
        self.modules.get(idx as usize)
    }

    pub fn get_module_mut(&mut self, idx: u32) -> Option<&mut AotModule> {
        self.modules.get_mut(idx as usize)
    }

    pub fn call(
        &self,
        module_idx: u32,
        func_idx: u32,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;
        module.call_native(func_idx, args)
    }

    pub fn memory_grow(&mut self, module_idx: u32, delta: u32) -> Result<i32> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(ref mut memory) = module.memory {
            let old_size = memory.size();
            memory.grow(delta)?;
            Ok(old_size as i32)
        } else {
            Err(WasmError::Runtime("no memory".into()))
        }
    }

    pub fn memory_size(&self, module_idx: u32) -> Result<i32> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(ref memory) = module.memory {
            Ok(memory.size() as i32)
        } else {
            Err(WasmError::Runtime("no memory".into()))
        }
    }

    pub fn table_grow(&mut self, module_idx: u32, table_idx: u32, delta: u32) -> Result<i32> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(ref mut table) = module.tables.get_mut(table_idx as usize) {
            let old_size = table.data.len() as u32;
            let new_size = old_size.saturating_add(delta);
            if let Some(max) = table.type_.limits.max() {
                if new_size > max {
                    return Ok(-1);
                }
            }
            table.data.resize(new_size as usize, 0);
            Ok(old_size as i32)
        } else {
            Err(WasmError::Runtime("table not found".into()))
        }
    }

    pub fn table_size(&self, module_idx: u32, table_idx: u32) -> Result<i32> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(table) = module.tables.get(table_idx as usize) {
            Ok(table.data.len() as i32)
        } else {
            Err(WasmError::Runtime("table not found".into()))
        }
    }

    pub fn get_global_value(&self, module_idx: u32, global_idx: u32) -> Result<WasmValue> {
        let module = self
            .get_module(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(global) = module.globals.get(global_idx as usize) {
            Ok(global.value)
        } else {
            Err(WasmError::Runtime("global not found".into()))
        }
    }

    pub fn set_global_value(
        &mut self,
        module_idx: u32,
        global_idx: u32,
        value: WasmValue,
    ) -> Result<()> {
        let module = self
            .get_module_mut(module_idx)
            .ok_or_else(|| WasmError::Runtime(format!("module {} not found", module_idx)))?;

        if let Some(ref mut global) = module.globals.get_mut(global_idx as usize) {
            global.value = value;
            Ok(())
        } else {
            Err(WasmError::Runtime("global not found".into()))
        }
    }
}

impl Default for AotRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_aot_module_from_wasm(module: &Module) -> AotModule {
    AotModule::from_module(module)
}

pub fn validate_aot_data(data: &[u8]) -> Result<()> {
    if data.len() < 4 {
        return Err(WasmError::Load("AOT data too short".to_string()));
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if magic != 0x6D736100 {
        return Err(WasmError::Load(format!("Invalid AOT magic: {:x}", magic)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RefType;
    use crate::runtime::{GlobalType, Limits, NumType, TableType, ValType};

    #[test]
    fn test_aot_module_creation() {
        let module = AotModule::from_module(&Module::new());
        assert_eq!(module.native_functions.len(), 0);
        assert!(module.memory.is_none());
    }

    #[test]
    fn test_native_registration() {
        let mut module = AotModule::from_module(&Module::new());
        let idx = module.register_native(Box::new(|_| Ok(vec![])));
        assert_eq!(idx, 0);
        assert_eq!(module.native_functions.len(), 1);
    }

    #[test]
    fn test_native_call() {
        let mut module = AotModule::from_module(&Module::new());
        module.register_native(Box::new(|args| {
            let a = args
                .first()
                .and_then(|v| match v {
                    WasmValue::I32(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            let b = args
                .get(1)
                .and_then(|v| match v {
                    WasmValue::I32(i) => Some(*i),
                    _ => None,
                })
                .unwrap_or(0);
            Ok(vec![WasmValue::I32(a + b)])
        }));

        let result = module
            .call_native(0, &[WasmValue::I32(5), WasmValue::I32(3)])
            .unwrap();
        assert_eq!(result, vec![WasmValue::I32(8)]);
    }

    #[test]
    fn test_table_management() {
        let mut module = AotModule::from_module(&Module::new());
        let table = Table::new(TableType::new(RefType::FuncRef, Limits::Min(10)));
        let idx = module.add_table(table);
        assert_eq!(idx, 0);
        assert!(module.get_table(0).is_some());
    }

    #[test]
    fn test_global_management() {
        let mut module = AotModule::from_module(&Module::new());
        let global = Global::new(
            GlobalType::new(ValType::Num(NumType::I32), true),
            WasmValue::I32(42),
        )
        .unwrap();
        let idx = module.add_global(global);
        assert_eq!(idx, 0);
        assert!(module.get_global(0).is_some());
    }

    #[test]
    fn test_runtime() {
        let runtime = AotRuntime::new();
        assert_eq!(runtime.modules.len(), 0);
    }

    #[test]
    fn test_validate_aot_data() {
        let valid_data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        assert!(validate_aot_data(&valid_data).is_ok());

        let invalid_data = vec![0x00, 0x00, 0x00, 0x00];
        assert!(validate_aot_data(&invalid_data).is_err());

        let short_data = vec![0x00, 0x61];
        assert!(validate_aot_data(&short_data).is_err());
    }
}
