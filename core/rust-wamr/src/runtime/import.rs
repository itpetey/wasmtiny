#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub module: String,
    pub name: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    Func(u32),
    Table(crate::runtime::TableType),
    Memory(crate::runtime::MemoryType),
    Global(crate::runtime::GlobalType),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportType {
    pub module: String,
    pub name: String,
    pub type_: ImportKind,
}

impl ImportType {
    pub fn new(module: String, name: String, kind: ImportKind) -> Self {
        Self {
            module,
            name,
            type_: kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import() {
        let import = Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(crate::runtime::MemoryType::new(
                crate::runtime::Limits::Min(1),
            )),
        };
        assert_eq!(import.module, "env");
        assert_eq!(import.name, "memory");
    }
}
