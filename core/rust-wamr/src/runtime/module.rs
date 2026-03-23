use super::{ExportType, FunctionType, GlobalType, Import, ImportKind, MemoryType, TableType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Module {
    pub types: Vec<FunctionType>,
    pub funcs: Vec<Func>,
    pub tables: Vec<TableType>,
    pub memories: Vec<MemoryType>,
    pub globals: Vec<GlobalType>,
    pub exports: Vec<ExportType>,
    pub imports: Vec<Import>,
    pub start: Option<u32>,
    pub data: Vec<DataSegment>,
    pub elems: Vec<ElemSegment>,
    names: HashMap<String, NameSection>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub type_idx: u32,
    pub locals: Vec<Local>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub count: u32,
    pub type_: super::ValType,
}

#[derive(Debug, Clone)]
pub struct DataSegment {
    pub kind: DataKind,
    pub init: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum DataKind {
    Active { memory_idx: u32, offset: Vec<u8> },
    Passive,
}

#[derive(Debug, Clone)]
pub struct ElemSegment {
    pub kind: ElemKind,
    pub type_: super::RefType,
}

#[derive(Debug, Clone)]
pub enum ElemKind {
    Active { table_idx: u32, offset: Vec<u8> },
    Passive,
    Declarative,
}

#[derive(Debug, Clone)]
pub struct NameSection {
    pub module_name: Option<String>,
    pub func_names: HashMap<u32, String>,
    pub local_names: HashMap<u32, HashMap<u32, String>>,
}

impl Module {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            funcs: Vec::new(),
            tables: Vec::new(),
            memories: Vec::new(),
            globals: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            start: None,
            data: Vec::new(),
            elems: Vec::new(),
            names: HashMap::new(),
        }
    }

    pub fn type_at(&self, idx: u32) -> Option<&FunctionType> {
        self.types.get(idx as usize)
    }

    pub fn func_at(&self, idx: u32) -> Option<&Func> {
        self.funcs.get(idx as usize)
    }

    pub fn table_at(&self, idx: u32) -> Option<&TableType> {
        self.tables.get(idx as usize)
    }

    pub fn memory_at(&self, idx: u32) -> Option<&MemoryType> {
        self.memories.get(idx as usize)
    }

    pub fn global_at(&self, idx: u32) -> Option<&GlobalType> {
        self.globals.get(idx as usize)
    }

    pub fn func_type(&self, func_idx: u32) -> Option<&FunctionType> {
        let func = self.func_at(func_idx)?;
        self.type_at(func.type_idx)
    }

    pub fn export(&self, name: &str) -> Option<&ExportType> {
        self.exports.iter().find(|e| e.name == name)
    }

    pub fn func_count(&self) -> u32 {
        let import_count = self
            .imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Func(_)))
            .count() as u32;
        import_count + self.funcs.len() as u32
    }

    pub fn import_count(&self) -> usize {
        self.imports.len()
    }

    pub fn get_func_imports(&self) -> Vec<&Import> {
        self.imports
            .iter()
            .filter(|i| matches!(i.kind, ImportKind::Func(_)))
            .collect()
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportType, Func, FunctionType, Module};
    use crate::{NumType, ValType};

    #[test]
    fn test_module_creation() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module
            .exports
            .push(ExportType::new_func("add".to_string(), 0));

        assert_eq!(module.types.len(), 1);
        assert_eq!(module.export("add").unwrap().name, "add");
    }

    #[test]
    fn test_func_type_lookup() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![],
        });

        assert!(module.func_type(0).is_some());
        assert!(module.func_type(1).is_none());
    }
}
