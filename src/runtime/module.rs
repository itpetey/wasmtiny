use super::{ExportType, FunctionType, GlobalType, Import, ImportKind, MemoryType, TableType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Module {
    pub types: Vec<FunctionType>,
    pub funcs: Vec<Func>,
    pub tables: Vec<TableType>,
    pub memories: Vec<MemoryType>,
    pub globals: Vec<GlobalType>,
    pub global_inits: Vec<Vec<u8>>,
    pub exports: Vec<ExportType>,
    pub imports: Vec<Import>,
    pub start: Option<u32>,
    pub data: Vec<DataSegment>,
    pub elems: Vec<ElemSegment>,
    #[allow(dead_code)]
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
    pub init: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub enum ElemKind {
    Active { table_idx: u32, offset: Vec<u8> },
    Passive,
    Declarative,
}

#[derive(Debug, Clone)]
pub struct NameSection {
    #[allow(dead_code)]
    pub module_name: Option<String>,
    #[allow(dead_code)]
    pub func_names: HashMap<u32, String>,
    #[allow(dead_code)]
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
            global_inits: Vec::new(),
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
        let import_func_count = self
            .imports
            .iter()
            .filter(|import| matches!(import.kind, ImportKind::Func(_)))
            .count() as u32;
        let local_idx = idx.checked_sub(import_func_count)?;
        self.defined_func_at(local_idx)
    }

    pub fn table_at(&self, idx: u32) -> Option<&TableType> {
        let mut import_table_count = 0u32;
        for import in &self.imports {
            if let ImportKind::Table(ref table_type) = import.kind {
                if import_table_count == idx {
                    return Some(table_type);
                }
                import_table_count += 1;
            }
        }

        let local_idx = idx.checked_sub(import_table_count)?;
        self.tables.get(local_idx as usize)
    }

    pub fn memory_at(&self, idx: u32) -> Option<&MemoryType> {
        let mut import_memory_count = 0u32;
        for import in &self.imports {
            if let ImportKind::Memory(ref memory_type) = import.kind {
                if import_memory_count == idx {
                    return Some(memory_type);
                }
                import_memory_count += 1;
            }
        }

        let local_idx = idx.checked_sub(import_memory_count)?;
        self.memories.get(local_idx as usize)
    }

    pub fn global_at(&self, idx: u32) -> Option<&GlobalType> {
        let mut import_global_count = 0u32;
        for import in &self.imports {
            if let ImportKind::Global(ref global_type) = import.kind {
                if import_global_count == idx {
                    return Some(global_type);
                }
                import_global_count += 1;
            }
        }

        let local_idx = idx.checked_sub(import_global_count)?;
        self.globals.get(local_idx as usize)
    }

    pub fn func_type(&self, func_idx: u32) -> Option<&FunctionType> {
        let mut import_func_count = 0u32;
        for import in &self.imports {
            if let ImportKind::Func(type_idx) = import.kind {
                if import_func_count == func_idx {
                    return self.type_at(type_idx);
                }
                import_func_count += 1;
            }
        }

        let local_idx = func_idx.checked_sub(import_func_count)?;
        let func = self.defined_func_at(local_idx)?;
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

    pub(crate) fn defined_func_at(&self, idx: u32) -> Option<&Func> {
        self.funcs.get(idx as usize)
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExportType, Func, FunctionType, GlobalType, Import, ImportKind, MemoryType, Module,
        TableType,
    };
    use crate::runtime::Limits;
    use crate::{NumType, RefType, ValType};

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

    #[test]
    fn test_func_accessor_uses_combined_index_space() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "imported".to_string(),
            kind: ImportKind::Func(0),
        });
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });

        assert!(module.func_at(0).is_none());
        assert_eq!(module.func_at(1).unwrap().type_idx, 0);
        assert_eq!(module.defined_func_at(0).unwrap().type_idx, 0);
    }

    #[test]
    fn test_imported_type_accessors_use_combined_index_space() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "table".to_string(),
            kind: ImportKind::Table(TableType::new(RefType::FuncRef, Limits::Min(1))),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryType::new(Limits::Min(2))),
        });
        module.imports.push(Import {
            module: "env".to_string(),
            name: "global".to_string(),
            kind: ImportKind::Global(GlobalType::new(ValType::Num(NumType::I32), false)),
        });
        module
            .tables
            .push(TableType::new(RefType::ExternRef, Limits::Min(3)));
        module.memories.push(MemoryType::new(Limits::Min(4)));
        module
            .globals
            .push(GlobalType::new(ValType::Num(NumType::I64), true));

        assert_eq!(module.table_at(0).unwrap().elem_type, RefType::FuncRef);
        assert_eq!(module.table_at(1).unwrap().elem_type, RefType::ExternRef);
        assert_eq!(module.memory_at(0).unwrap().limits.min(), 2);
        assert_eq!(module.memory_at(1).unwrap().limits.min(), 4);
        assert_eq!(
            module.global_at(0).unwrap().content_type,
            ValType::Num(NumType::I32)
        );
        assert_eq!(
            module.global_at(1).unwrap().content_type,
            ValType::Num(NumType::I64)
        );
    }
}
