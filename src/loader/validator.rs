use crate::runtime::{Module, NumType, RefType, Result, ValType, WasmError};

pub struct Validator;

impl Validator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, module: &Module) -> Result<()> {
        self.validate_types(module)?;
        self.validate_functions(module)?;
        self.validate_tables(module)?;
        self.validate_memories(module)?;
        self.validate_globals(module)?;
        self.validate_exports(module)?;
        self.validate_start(module)?;
        Ok(())
    }

    fn validate_types(&self, module: &Module) -> Result<()> {
        for (i, func_type) in module.types.iter().enumerate() {
            if func_type.params.len() > 16 {
                return Err(WasmError::Validation(format!(
                    "type {}: too many parameters",
                    i
                )));
            }
        }
        Ok(())
    }

    fn validate_functions(&self, module: &Module) -> Result<()> {
        for (i, func) in module.funcs.iter().enumerate() {
            if func.type_idx as usize >= module.types.len() {
                return Err(WasmError::Validation(format!(
                    "function {}: invalid type index",
                    i
                )));
            }
        }
        Ok(())
    }

    fn validate_tables(&self, module: &Module) -> Result<()> {
        for (i, table) in module.tables.iter().enumerate() {
            if table.limits.min() > 0x10000000 {
                return Err(WasmError::Validation(format!(
                    "table {}: minimum size too large",
                    i
                )));
            }
            if let Some(max) = table.limits.max() {
                if max > 0x10000000 {
                    return Err(WasmError::Validation(format!(
                        "table {}: maximum size too large",
                        i
                    )));
                }
                if max < table.limits.min() {
                    return Err(WasmError::Validation(format!(
                        "table {}: maximum less than minimum",
                        i
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_memories(&self, module: &Module) -> Result<()> {
        for (i, memory) in module.memories.iter().enumerate() {
            if memory.limits.min() > 65536 {
                return Err(WasmError::Validation(format!(
                    "memory {}: minimum size too large",
                    i
                )));
            }
            if let Some(max) = memory.limits.max() {
                if max > 65536 {
                    return Err(WasmError::Validation(format!(
                        "memory {}: maximum size too large",
                        i
                    )));
                }
                if max < memory.limits.min() {
                    return Err(WasmError::Validation(format!(
                        "memory {}: maximum less than minimum",
                        i
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_globals(&self, module: &Module) -> Result<()> {
        for (i, global) in module.globals.iter().enumerate() {
            if !matches!(global.content_type, ValType::Num(_))
                && !matches!(global.content_type, ValType::Ref(_))
            {
                return Err(WasmError::Validation(format!("global {}: invalid type", i)));
            }
        }
        Ok(())
    }

    fn validate_exports(&self, module: &Module) -> Result<()> {
        for (i, export) in module.exports.iter().enumerate() {
            match &export.kind {
                crate::runtime::ExportKind::Func(idx) => {
                    if *idx as usize
                        >= module.funcs.len()
                            + module
                                .imports
                                .iter()
                                .filter(|i| matches!(i.kind, crate::runtime::ImportKind::Func(_)))
                                .count()
                    {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid function index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Table(idx) => {
                    if *idx as usize >= module.tables.len() {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid table index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Memory(idx) => {
                    if *idx as usize >= module.memories.len() {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid memory index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Global(idx) => {
                    if *idx as usize >= module.globals.len() {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid global index",
                            i
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_start(&self, module: &Module) -> Result<()> {
        if let Some(start_idx) = module.start {
            let import_count = module
                .imports
                .iter()
                .filter(|i| matches!(i.kind, crate::runtime::ImportKind::Func(_)))
                .count() as u32;
            if start_idx >= import_count + module.funcs.len() as u32 {
                return Err(WasmError::Validation(
                    "start function index out of bounds".to_string(),
                ));
            }
            let func_type = module.func_type(start_idx).ok_or_else(|| {
                WasmError::Validation("start function has invalid type".to_string())
            })?;
            if !func_type.params.is_empty() || !func_type.results.is_empty() {
                return Err(WasmError::Validation(
                    "start function must have no params or results".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_module() {
        let module = Module::new();
        let validator = Validator::new();
        assert!(validator.validate(&module).is_ok());
    }
}
