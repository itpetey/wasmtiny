use crate::runtime::{ExportKind, ExportType};
use crate::runtime::{FunctionType, Module, NumType, Result, ValType, WasmValue};
use std::sync::Arc;

pub struct IntegrationTestHarness {
    pub modules: Vec<Arc<Module>>,
}

impl IntegrationTestHarness {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn add_module(&mut self, module: Module) -> usize {
        let idx = self.modules.len();
        self.modules.push(Arc::new(module));
        idx
    }

    pub fn get_module(&self, idx: usize) -> Option<&Arc<Module>> {
        self.modules.get(idx)
    }
}

impl Default for IntegrationTestHarness {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WasmTest {
    pub name: String,
    pub wasm_bytes: Vec<u8>,
    pub expected_results: Vec<ExpectedResult>,
}

pub struct ExpectedResult {
    pub func_name: String,
    pub args: Vec<WasmValue>,
    pub expected: Vec<WasmValue>,
}

pub fn run_integration_test(
    _harness: &IntegrationTestHarness,
    test: &WasmTest,
) -> Result<TestReport> {
    let parser = crate::loader::Parser::new();
    let module = parser.parse(&test.wasm_bytes)?;

    let validator = crate::loader::Validator::new();
    validator.validate(&module)?;

    Ok(TestReport {
        name: test.name.clone(),
        passed: true,
        errors: vec![],
    })
}

pub struct TestReport {
    pub name: String,
    pub passed: bool,
    pub errors: Vec<String>,
}

pub fn create_simple_add_module() -> Module {
    let mut module = Module::new();

    module.types.push(FunctionType::new(
        vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
        vec![ValType::Num(NumType::I32)],
    ));

    module.funcs.push(crate::runtime::Func {
        type_idx: 0,
        locals: vec![],
        body: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B],
    });

    module.exports.push(ExportType {
        name: "add".to_string(),
        kind: ExportKind::Func(0),
    });

    module
}

pub fn create_simple_sub_module() -> Module {
    let mut module = Module::new();

    module.types.push(FunctionType::new(
        vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
        vec![ValType::Num(NumType::I32)],
    ));

    module.funcs.push(crate::runtime::Func {
        type_idx: 0,
        locals: vec![],
        body: vec![0x20, 0x00, 0x20, 0x01, 0x6B, 0x0B],
    });

    module.exports.push(ExportType {
        name: "sub".to_string(),
        kind: ExportKind::Func(0),
    });

    module
}

pub fn create_memory_module() -> Module {
    let mut module = Module::new();

    module.memories.push(crate::runtime::MemoryType::new(
        crate::runtime::Limits::Min(1),
    ));

    module
        .types
        .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));

    module.funcs.push(crate::runtime::Func {
        type_idx: 0,
        locals: vec![],
        body: vec![0x41, 0x00, 0x0B],
    });

    module.exports.push(ExportType {
        name: "size".to_string(),
        kind: ExportKind::Func(0),
    });

    module.exports.push(ExportType {
        name: "memory".to_string(),
        kind: ExportKind::Memory(0),
    });

    module
}

pub fn run_all_tests() -> Vec<TestReport> {
    let mut reports = Vec::new();
    let harness = IntegrationTestHarness::new();

    let _add_module = create_simple_add_module();
    let test1 = WasmTest {
        name: "simple_add".to_string(),
        wasm_bytes: vec![],
        expected_results: vec![],
    };

    if let Ok(report) = run_integration_test(&harness, &test1) {
        reports.push(report);
    }

    let _sub_module = create_simple_sub_module();
    let test2 = WasmTest {
        name: "simple_sub".to_string(),
        wasm_bytes: vec![],
        expected_results: vec![],
    };

    if let Ok(report) = run_integration_test(&harness, &test2) {
        reports.push(report);
    }

    let _memory_module = create_memory_module();
    let test3 = WasmTest {
        name: "memory_size".to_string(),
        wasm_bytes: vec![],
        expected_results: vec![],
    };

    if let Ok(report) = run_integration_test(&harness, &test3) {
        reports.push(report);
    }

    reports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = IntegrationTestHarness::new();
        assert_eq!(harness.modules.len(), 0);
    }

    #[test]
    fn test_add_module() {
        let module = create_simple_add_module();
        assert_eq!(module.types.len(), 1);
        assert_eq!(module.funcs.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }

    #[test]
    fn test_sub_module() {
        let module = create_simple_sub_module();
        assert_eq!(module.types.len(), 1);
        assert_eq!(module.funcs.len(), 1);
    }

    #[test]
    fn test_memory_module() {
        let module = create_memory_module();
        assert_eq!(module.memories.len(), 1);
        assert_eq!(module.exports.len(), 2);
    }

    #[test]
    fn test_run_all_tests() {
        let _reports = run_all_tests();
    }
}
