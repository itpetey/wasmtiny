#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportType {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

impl ExportType {
    pub fn new_func(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Func(idx),
        }
    }

    pub fn new_table(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Table(idx),
        }
    }

    pub fn new_memory(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Memory(idx),
        }
    }

    pub fn new_global(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Global(idx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_func() {
        let export = ExportType::new_func("add".to_string(), 0);
        assert_eq!(export.name, "add");
        assert!(matches!(export.kind, ExportKind::Func(0)));
    }
}
