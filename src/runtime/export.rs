/// Export type descriptor.
///
/// Describes an exported WebAssembly entity with its name and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Export type.
pub struct ExportType {
    /// The name of the export.
    pub name: String,
    /// The kind of the export (function, table, memory, global, or tag).
    pub kind: ExportKind,
}

/// The kind of an export.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Export kind.
pub enum ExportKind {
    /// A function export (index into function section).
    Func(u32),
    /// A table export (index into table section).
    Table(u32),
    /// A memory export (index into memory section).
    Memory(u32),
    /// A global export (index into global section).
    Global(u32),
    /// A tag export (index into tag section).
    Tag(u32),
}

impl ExportType {
    /// Creates a function export descriptor.
    pub fn new_func(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Func(idx),
        }
    }

    /// Creates a table export descriptor.
    pub fn new_table(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Table(idx),
        }
    }

    /// Creates a memory export descriptor.
    pub fn new_memory(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Memory(idx),
        }
    }

    /// Creates a global export descriptor.
    pub fn new_global(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Global(idx),
        }
    }

    /// Creates a tag export descriptor.
    pub fn new_tag(name: String, idx: u32) -> Self {
        Self {
            name,
            kind: ExportKind::Tag(idx),
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
