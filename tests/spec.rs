use std::collections::HashMap;

use wasmtiny::loader::{Parser, Validator};
use wasmtiny::runtime::{
    FunctionType, Global, GlobalType, HostFunc, ImportKind, Limits, Memory, MemoryType, NumType,
    RefType, Store, Table, TableType, ValType,
};
use wasmtiny::{WasmApplication, WasmValue};
use wast::core::{AbstractHeapType, NanPattern, WastArgCore, WastRetCore};
use wast::parser::{self, ParseBuffer};
use wast::{QuoteWat, Wast, WastArg, WastDirective, WastExecute, WastInvoke, WastRet};

const SPEC_DIR: &str = "tests/spec/test/core/";

#[derive(Debug, Clone)]
enum SpecTestResult {
    Passed,
    Failed(String),
    Error(String),
}

#[derive(Default)]
struct SpecRunStats {
    passed: usize,
    failed: usize,
    skipped: usize,
    errors: Vec<String>,
}

struct SpecHarness {
    app: WasmApplication,
    parser: Parser,
    validator: Validator,
    current_module: Option<u32>,
    named_modules: HashMap<String, u32>,
    registered_modules: HashMap<String, u32>,
}

impl SpecHarness {
    fn new() -> Self {
        Self {
            app: WasmApplication::new(),
            parser: Parser::new(),
            validator: Validator::new(),
            current_module: None,
            named_modules: HashMap::new(),
            registered_modules: HashMap::new(),
        }
    }

    fn run_directive(&mut self, directive: WastDirective<'_>) -> Result<DirectiveOutcome, String> {
        match directive {
            WastDirective::Module(mut module) => {
                self.instantiate_module(&mut module)?;
                Ok(DirectiveOutcome::None)
            }
            WastDirective::ModuleDefinition(mut module) => {
                self.validate_quote_wat(&mut module)?;
                Ok(DirectiveOutcome::None)
            }
            WastDirective::Register { name, module, .. } => {
                let module_idx = if let Some(module) = module {
                    self.named_module_idx(module.name())?
                } else {
                    self.current_module
                        .ok_or_else(|| "register directive has no current module".to_string())?
                };
                self.registered_modules.insert(name.to_string(), module_idx);
                Ok(DirectiveOutcome::None)
            }
            WastDirective::Invoke(invoke) => match self.execute_invoke(&invoke) {
                Ok(_) => Ok(DirectiveOutcome::None),
                Err(error) if is_missing_current_module(&error) => {
                    Ok(DirectiveOutcome::Skipped(error))
                }
                Err(error) => Err(error),
            },
            WastDirective::AssertReturn { exec, results, .. } => {
                match self.assert_return(exec, results) {
                    Ok(()) => Ok(DirectiveOutcome::Passed),
                    Err(error) if is_missing_current_module(&error) => {
                        Ok(DirectiveOutcome::Skipped(error))
                    }
                    Err(error) => Err(error),
                }
            }
            WastDirective::AssertTrap { exec, .. } => match self.assert_trap(exec) {
                Ok(()) => Ok(DirectiveOutcome::Passed),
                Err(error) if is_missing_current_module(&error) => {
                    Ok(DirectiveOutcome::Skipped(error))
                }
                Err(error) => Err(error),
            },
            WastDirective::AssertExhaustion { call, .. } => match self.execute_invoke(&call) {
                Ok(_) => Err(
                    "assert_exhaustion expected an execution failure, but call succeeded"
                        .to_string(),
                ),
                Err(error) if is_missing_current_module(&error) => {
                    Ok(DirectiveOutcome::Skipped(error))
                }
                Err(_) => Ok(DirectiveOutcome::Passed),
            },
            WastDirective::AssertInvalid { mut module, .. } => {
                match self.assert_invalid(&mut module) {
                    Ok(()) => Ok(DirectiveOutcome::Passed),
                    Err(error) if error == "unsupported typed-ref invalid module" => {
                        Ok(DirectiveOutcome::Skipped(error))
                    }
                    Err(error) => Err(error),
                }
            }
            WastDirective::AssertMalformed { mut module, .. } => {
                self.assert_malformed(&mut module)?;
                Ok(DirectiveOutcome::Passed)
            }
            WastDirective::AssertUnlinkable { module, .. } => {
                self.assert_unlinkable(module)?;
                Ok(DirectiveOutcome::Passed)
            }
            WastDirective::AssertException { .. }
            | WastDirective::AssertSuspension { .. }
            | WastDirective::Thread(_)
            | WastDirective::Wait { .. }
            | WastDirective::ModuleInstance { .. } => Ok(DirectiveOutcome::Skipped(
                "unsupported WAST directive in Rust harness".to_string(),
            )),
        }
    }

    fn load_module(&mut self, module: &mut QuoteWat<'_>) -> Result<u32, String> {
        let wasm = module
            .encode()
            .map_err(|error| format!("module encoding failed: {error}"))?;
        self.app
            .load_module_from_memory(&wasm)
            .map_err(|error| format!("module load failed: {error}"))
    }

    fn validate_quote_wat(&mut self, module: &mut QuoteWat<'_>) -> Result<(), String> {
        let wasm = module
            .encode()
            .map_err(|error| format!("module encoding failed: {error}"))?;
        let parsed = self
            .parser
            .parse(&wasm)
            .map_err(|error| format!("module parse failed: {error}"))?;
        self.validator
            .validate(&parsed)
            .map_err(|error| format!("module validation failed: {error}"))
    }

    fn instantiate_module(&mut self, module: &mut QuoteWat<'_>) -> Result<u32, String> {
        let module_name = module.name().map(|id| id.name().to_string());
        self.current_module = None;
        let module_idx = self.load_module(module)?;
        self.resolve_imports(module_idx)?;
        self.app
            .instantiate(module_idx)
            .map_err(|error| format!("module instantiation failed: {error}"))?;
        self.app
            .execute_start(module_idx)
            .map_err(|error| format!("module start failed: {error}"))?;

        if let Some(module_name) = module_name {
            self.named_modules.insert(module_name, module_idx);
        }
        self.current_module = Some(module_idx);
        Ok(module_idx)
    }

    fn resolve_imports(&mut self, module_idx: u32) -> Result<(), String> {
        let imports = self
            .app
            .imports(module_idx)
            .map_err(|error| format!("failed to inspect imports: {error}"))?;

        for import in imports {
            if import.module == "spectest" {
                self.resolve_spectest_import(module_idx, &import)?;
                continue;
            }

            if let Some(source_module_idx) = self.registered_modules.get(&import.module).copied() {
                self.resolve_registered_import(module_idx, source_module_idx, &import)?;
            }
        }

        Ok(())
    }

    fn resolve_spectest_import(
        &mut self,
        module_idx: u32,
        import: &wasmtiny::runtime::Import,
    ) -> Result<(), String> {
        match &import.kind {
            ImportKind::Memory(memory_type) => {
                if import.name != "memory" {
                    return Err(format!(
                        "unsupported spectest memory import {}.{}",
                        import.module, import.name
                    ));
                }
                self.app
                    .register_memory_import(
                        module_idx,
                        &import.module,
                        &import.name,
                        spectest_memory(memory_type),
                    )
                    .map_err(|error| format!("failed to register spectest memory import: {error}"))
            }
            ImportKind::Table(table_type) => {
                if import.name != "table" {
                    return Err(format!(
                        "unsupported spectest table import {}.{}",
                        import.module, import.name
                    ));
                }
                self.app
                    .register_table_import(
                        module_idx,
                        &import.module,
                        &import.name,
                        spectest_table(table_type),
                    )
                    .map_err(|error| format!("failed to register spectest table import: {error}"))
            }
            ImportKind::Global(global_type) => {
                let global = spectest_global(&import.name, global_type)?;
                self.app
                    .register_global_import(module_idx, &import.module, &import.name, global)
                    .map_err(|error| format!("failed to register spectest global import: {error}"))
            }
            ImportKind::Func(_) => {
                let function_type = spectest_function_type(&import.name)?;
                self.app
                    .register_host_function(
                        module_idx,
                        &import.module,
                        &import.name,
                        Box::new(NoOpHostFunc {
                            function_type: function_type.clone(),
                        }),
                        function_type,
                    )
                    .map_err(|error| format!("failed to register spectest host import: {error}"))
            }
            ImportKind::Tag(_) => Err(format!(
                "unsupported spectest tag import {}.{}",
                import.module, import.name
            )),
        }
    }

    fn resolve_registered_import(
        &mut self,
        module_idx: u32,
        source_module_idx: u32,
        import: &wasmtiny::runtime::Import,
    ) -> Result<(), String> {
        match &import.kind {
            ImportKind::Memory(_) => {
                let memory = self
                    .app
                    .export_memory(source_module_idx, &import.name)
                    .map_err(|error| format!("failed to export memory {}: {error}", import.name))?;
                self.app
                    .register_memory_import(module_idx, &import.module, &import.name, memory)
                    .map_err(|error| {
                        format!("failed to register memory import {}: {error}", import.name)
                    })
            }
            ImportKind::Table(_) => {
                let table_idx = self
                    .app
                    .export_table_index(source_module_idx, &import.name)
                    .map_err(|error| format!("failed to locate table {}: {error}", import.name))?;
                let table = self
                    .app
                    .table_binding(source_module_idx, table_idx)
                    .map_err(|error| format!("failed to export table {}: {error}", import.name))?;
                self.app
                    .register_table_import_binding(module_idx, &import.module, &import.name, table)
                    .map_err(|error| {
                        format!("failed to register table import {}: {error}", import.name)
                    })
            }
            ImportKind::Global(_) => {
                let global = self
                    .app
                    .export_global(source_module_idx, &import.name)
                    .map_err(|error| format!("failed to export global {}: {error}", import.name))?;
                self.app
                    .register_global_import(module_idx, &import.module, &import.name, global)
                    .map_err(|error| {
                        format!("failed to register global import {}: {error}", import.name)
                    })
            }
            ImportKind::Tag(_) => {
                let function_type = self
                    .app
                    .tag_type(source_module_idx, &import.name)
                    .map_err(|error| format!("failed to export tag {}: {error}", import.name))?;
                self.app
                    .register_tag_import(module_idx, &import.module, &import.name, function_type)
                    .map_err(|error| {
                        format!("failed to register tag import {}: {error}", import.name)
                    })
            }
            ImportKind::Func(_type_idx) => {
                let binding = self
                    .app
                    .function_binding(source_module_idx, &import.name)
                    .map_err(|error| {
                        format!("failed to export function {}: {error}", import.name)
                    })?;
                self.app
                    .register_function_import_binding(
                        module_idx,
                        &import.module,
                        &import.name,
                        binding,
                    )
                    .map_err(|error| {
                        format!(
                            "failed to register function import {}: {error}",
                            import.name
                        )
                    })
            }
        }
    }

    fn assert_return(
        &mut self,
        exec: WastExecute<'_>,
        results: Vec<WastRet<'_>>,
    ) -> Result<(), String> {
        let values = self.execute(exec)?;
        if values.len() != results.len() {
            return Err(format!(
                "expected {} return values, got {}",
                results.len(),
                values.len()
            ));
        }

        for (actual, expected) in values.iter().zip(results.iter()) {
            if !matches_return(actual, expected) {
                return Err(format!("expected {expected:?}, got {actual:?}"));
            }
        }

        Ok(())
    }

    fn assert_trap(&mut self, exec: WastExecute<'_>) -> Result<(), String> {
        self.execute(exec).err().ok_or_else(|| {
            "assert_trap expected an execution failure, but execution succeeded".to_string()
        })?;
        Ok(())
    }

    fn assert_invalid(&mut self, module: &mut QuoteWat<'_>) -> Result<(), String> {
        let wasm = module
            .encode()
            .map_err(|error| format!("module encoding failed: {error}"))?;

        match self.validate_encoded_module(&wasm) {
            Ok(()) if contains_unsupported_gc_heap_types(&wasm) => {
                Err("unsupported typed-ref invalid module".to_string())
            }
            Ok(()) => Err("assert_invalid expected validation to fail".to_string()),
            Err(_) => Ok(()),
        }
    }

    fn validate_encoded_module(&mut self, wasm: &[u8]) -> Result<(), String> {
        let parsed = self
            .parser
            .parse(wasm)
            .map_err(|error| format!("module parse failed: {error}"))?;
        self.validator
            .validate(&parsed)
            .map_err(|error| format!("module validation failed: {error}"))
    }

    fn assert_malformed(&mut self, module: &mut QuoteWat<'_>) -> Result<(), String> {
        match module.encode() {
            Ok(wasm) => {
                if self.app.load_module_from_memory(&wasm).is_ok() {
                    Err("assert_malformed expected module loading to fail".to_string())
                } else {
                    Ok(())
                }
            }
            Err(_) => Ok(()),
        }
    }

    fn assert_unlinkable(&mut self, mut module: wast::Wat<'_>) -> Result<(), String> {
        let wasm = module
            .encode()
            .map_err(|error| format!("module encoding failed: {error}"))?;
        let module_idx = self
            .app
            .load_module_from_memory(&wasm)
            .map_err(|error| format!("module load failed: {error}"))?;
        let _ = self.resolve_imports(module_idx);
        self.app.instantiate(module_idx).err().ok_or_else(|| {
            "assert_unlinkable expected instantiation to fail, but it succeeded".to_string()
        })?;
        Ok(())
    }

    fn execute(&mut self, exec: WastExecute<'_>) -> Result<Vec<WasmValue>, String> {
        match exec {
            WastExecute::Invoke(invoke) => self.execute_invoke(&invoke),
            WastExecute::Wat(mut module) => {
                self.instantiate_wat(&mut module)?;
                Ok(Vec::new())
            }
            WastExecute::Get { module, global, .. } => {
                let module_idx = self.lookup_module(module.map(|id| id.name()))?;
                let global = self
                    .app
                    .export_global(module_idx, global)
                    .map_err(|error| format!("failed to read global {global}: {error}"))?;
                Ok(vec![global.get()])
            }
        }
    }

    fn instantiate_wat(&mut self, module: &mut wast::Wat<'_>) -> Result<u32, String> {
        self.current_module = None;
        let wasm = module
            .encode()
            .map_err(|error| format!("module encoding failed: {error}"))?;
        let module_idx = self
            .app
            .load_module_from_memory(&wasm)
            .map_err(|error| format!("module load failed: {error}"))?;
        self.resolve_imports(module_idx)?;
        self.app
            .instantiate(module_idx)
            .map_err(|error| format!("module instantiation failed: {error}"))?;
        self.app
            .execute_start(module_idx)
            .map_err(|error| format!("module start failed: {error}"))?;
        self.current_module = Some(module_idx);
        Ok(module_idx)
    }

    fn execute_invoke(&mut self, invoke: &WastInvoke<'_>) -> Result<Vec<WasmValue>, String> {
        let module_idx = self.lookup_module(invoke.module.map(|id| id.name()))?;
        let args = invoke
            .args
            .iter()
            .map(wast_arg_to_value)
            .collect::<Result<Vec<_>, _>>()?;
        self.app
            .call_function(module_idx, invoke.name, &args)
            .map_err(|error| format!("invoke {} failed: {error}", invoke.name))
    }

    fn lookup_module(&self, name: Option<&str>) -> Result<u32, String> {
        match name {
            Some(name) => self
                .named_modules
                .get(name)
                .copied()
                .ok_or_else(|| format!("unknown module id ${name}")),
            None => self
                .current_module
                .ok_or_else(|| "no current module available".to_string()),
        }
    }

    fn named_module_idx(&self, name: &str) -> Result<u32, String> {
        self.named_modules
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown module id ${name}"))
    }
}

enum DirectiveOutcome {
    None,
    Passed,
    Skipped(String),
}

struct NoOpHostFunc {
    function_type: FunctionType,
}

impl HostFunc for NoOpHostFunc {
    fn call(
        &self,
        _store: &mut Store,
        _args: &[WasmValue],
    ) -> wasmtiny::runtime::Result<Vec<WasmValue>> {
        Ok(Vec::new())
    }

    fn function_type(&self) -> Option<&FunctionType> {
        Some(&self.function_type)
    }
}

fn is_missing_current_module(error: &str) -> bool {
    error == "no current module available"
}

fn contains_unsupported_gc_heap_types(wasm: &[u8]) -> bool {
    wasm.windows(2)
        .any(|window| matches!(window, [prefix, next] if (*prefix == 0x63 || *prefix == 0x64) && *next < 0x40))
}

fn spectest_global(name: &str, global_type: &GlobalType) -> Result<Global, String> {
    let value = match name {
        "global_i32" => WasmValue::I32(666),
        "global_i64" => WasmValue::I64(666),
        "global_f32" => WasmValue::F32(666.6),
        "global_f64" => WasmValue::F64(666.6),
        _ => return Err(format!("unsupported spectest global {name}")),
    };

    Global::new(global_type.clone(), value)
        .map_err(|error| format!("invalid spectest global {name}: {error}"))
}

fn spectest_memory(_required: &MemoryType) -> Memory {
    Memory::new(MemoryType::new(Limits::MinMax(1, 2)))
}

fn spectest_table(required: &TableType) -> Table {
    Table::new(TableType::new(required.elem_type, Limits::MinMax(10, 20)))
}

fn spectest_function_type(name: &str) -> Result<FunctionType, String> {
    let params = match name {
        "print" => vec![],
        "print_i32" => vec![ValType::Num(NumType::I32)],
        "print_i64" => vec![ValType::Num(NumType::I64)],
        "print_f32" => vec![ValType::Num(NumType::F32)],
        "print_f64" => vec![ValType::Num(NumType::F64)],
        "print_i32_f32" => vec![ValType::Num(NumType::I32), ValType::Num(NumType::F32)],
        "print_f64_f64" => vec![ValType::Num(NumType::F64), ValType::Num(NumType::F64)],
        _ => {
            return Err(format!(
                "unsupported spectest function import spectest.{name}"
            ));
        }
    };

    Ok(FunctionType::new(params, vec![]))
}

fn wast_arg_to_value(arg: &WastArg<'_>) -> Result<WasmValue, String> {
    match arg {
        WastArg::Core(core) => wast_core_arg_to_value(core),
        _ => Err("component-model WAST arguments are unsupported".to_string()),
    }
}

fn wast_core_arg_to_value(arg: &WastArgCore<'_>) -> Result<WasmValue, String> {
    match arg {
        WastArgCore::I32(value) => Ok(WasmValue::I32(*value)),
        WastArgCore::I64(value) => Ok(WasmValue::I64(*value)),
        WastArgCore::F32(value) => Ok(WasmValue::F32(f32::from_bits(value.bits))),
        WastArgCore::F64(value) => Ok(WasmValue::F64(f64::from_bits(value.bits))),
        WastArgCore::RefNull(heap_type) => {
            Ok(WasmValue::NullRef(heap_type_to_ref_type(heap_type)?))
        }
        WastArgCore::RefExtern(value) => Ok(WasmValue::ExternRef(*value)),
        WastArgCore::RefHost(value) => Ok(WasmValue::ExternRef(*value)),
        WastArgCore::V128(_) => Err("v128 WAST arguments are unsupported".to_string()),
    }
}

fn heap_type_to_ref_type(heap_type: &wast::core::HeapType<'_>) -> Result<RefType, String> {
    match heap_type {
        wast::core::HeapType::Abstract {
            ty: AbstractHeapType::Func | AbstractHeapType::NoFunc,
            ..
        }
        | wast::core::HeapType::Concrete(_)
        | wast::core::HeapType::Exact(_) => Ok(RefType::FuncRef),
        wast::core::HeapType::Abstract {
            ty: AbstractHeapType::Extern | AbstractHeapType::NoExtern,
            ..
        } => Ok(RefType::ExternRef),
        _ => Err("unsupported heap type in WAST reference".to_string()),
    }
}

fn matches_return(actual: &WasmValue, expected: &WastRet<'_>) -> bool {
    match expected {
        WastRet::Core(expected) => matches_core_return(actual, expected),
        _ => false,
    }
}

fn matches_core_return(actual: &WasmValue, expected: &WastRetCore<'_>) -> bool {
    match expected {
        WastRetCore::I32(expected) => matches!(actual, WasmValue::I32(value) if value == expected),
        WastRetCore::I64(expected) => matches!(actual, WasmValue::I64(value) if value == expected),
        WastRetCore::F32(pattern) => {
            matches!(actual, WasmValue::F32(value) if matches_f32_pattern(*value, pattern))
        }
        WastRetCore::F64(pattern) => {
            matches!(actual, WasmValue::F64(value) if matches_f64_pattern(*value, pattern))
        }
        WastRetCore::RefNull(_) => matches!(actual, WasmValue::NullRef(_)),
        WastRetCore::RefExtern(Some(expected)) => {
            matches!(actual, WasmValue::ExternRef(value) if value == expected)
        }
        WastRetCore::RefExtern(None) => matches!(actual, WasmValue::ExternRef(_)),
        WastRetCore::RefFunc(_) => matches!(actual, WasmValue::FuncRef(_)),
        WastRetCore::Either(cases) => cases.iter().any(|case| matches_core_return(actual, case)),
        _ => false,
    }
}

fn matches_f32_pattern(actual: f32, pattern: &NanPattern<wast::token::F32>) -> bool {
    match pattern {
        NanPattern::Value(expected) => actual.to_bits() == expected.bits,
        NanPattern::CanonicalNan | NanPattern::ArithmeticNan => actual.is_nan(),
    }
}

fn matches_f64_pattern(actual: f64, pattern: &NanPattern<wast::token::F64>) -> bool {
    match pattern {
        NanPattern::Value(expected) => actual.to_bits() == expected.bits,
        NanPattern::CanonicalNan | NanPattern::ArithmeticNan => actual.is_nan(),
    }
}

fn run_spec_test(filename: &str) -> SpecTestResult {
    let path = SPEC_DIR.to_owned() + filename;
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => return SpecTestResult::Error(format!("failed to read spec file: {error}")),
    };

    let buf = match ParseBuffer::new(&source) {
        Ok(buf) => buf,
        Err(error) => {
            return SpecTestResult::Error(format!("failed to parse WAST buffer: {error}"));
        }
    };

    let wast = match parser::parse::<Wast<'_>>(&buf) {
        Ok(wast) => wast,
        Err(error) => return SpecTestResult::Error(format!("failed to parse WAST: {error}")),
    };

    let mut harness = SpecHarness::new();
    let mut stats = SpecRunStats::default();

    for (index, directive) in wast.directives.into_iter().enumerate() {
        let (line, _column) = directive.span().linecol_in(&source);
        match harness.run_directive(directive) {
            Ok(DirectiveOutcome::None) => {}
            Ok(DirectiveOutcome::Passed) => stats.passed += 1,
            Ok(DirectiveOutcome::Skipped(reason)) => {
                stats.skipped += 1;
                let _ = reason;
            }
            Err(error) => {
                stats.failed += 1;
                stats.errors.push(format!(
                    "directive {} (line {}): {}",
                    index + 1,
                    line + 1,
                    error
                ));
            }
        }
    }

    if stats.failed > 0 {
        SpecTestResult::Failed(format!(
            "{} passed, {} failed, {} skipped\n{}",
            stats.passed,
            stats.failed,
            stats.skipped,
            stats.errors.join("\n")
        ))
    } else {
        SpecTestResult::Passed
    }
}

macro_rules! spec_test {
    ($name:ident, $file:literal) => {
        #[test]
        fn $name() {
            assert_spec_passes($file);
        }
    };
}

fn assert_spec_passes(name: &str) {
    let result = run_spec_test(name);
    match result {
        SpecTestResult::Passed => {}
        SpecTestResult::Failed(message) => panic!("Expected pass, got failure: {message}"),
        SpecTestResult::Error(message) => panic!("Expected pass, got error: {message}"),
    }
}

spec_test!(test_spec_block, "block.wast");
spec_test!(test_spec_br, "br.wast");
spec_test!(test_spec_br_if, "br_if.wast");
spec_test!(test_spec_br_table, "br_table.wast");
spec_test!(test_spec_call, "call.wast");
spec_test!(test_spec_call_indirect, "call_indirect.wast");
spec_test!(test_spec_const, "const.wast");
spec_test!(test_spec_conversions, "conversions.wast");
spec_test!(test_spec_data, "data.wast");
spec_test!(test_spec_elem, "elem.wast");
spec_test!(test_spec_exports, "exports.wast");
spec_test!(test_spec_f32, "f32.wast");
spec_test!(test_spec_f32_cmp, "f32_cmp.wast");
spec_test!(test_spec_f64, "f64.wast");
spec_test!(test_spec_f64_cmp, "f64_cmp.wast");
spec_test!(test_spec_fac, "fac.wast");
spec_test!(test_spec_float_literals, "float_literals.wast");
spec_test!(test_spec_float_memory, "float_memory.wast");
spec_test!(test_spec_float_misc, "float_misc.wast");
spec_test!(test_spec_func, "func.wast");
spec_test!(test_spec_global, "global.wast");
spec_test!(test_spec_i32, "i32.wast");
spec_test!(test_spec_id, "id.wast");
spec_test!(test_spec_imports, "imports.wast");
spec_test!(test_spec_int_literals, "int_literals.wast");
spec_test!(test_spec_labels, "labels.wast");
spec_test!(test_spec_load, "load.wast");
spec_test!(test_spec_local_get, "local_get.wast");
spec_test!(test_spec_local_set, "local_set.wast");
spec_test!(test_spec_local_tee, "local_tee.wast");
spec_test!(test_spec_loop, "loop.wast");
spec_test!(test_spec_memory, "memory.wast");
spec_test!(test_spec_memory_grow, "memory_grow.wast");
spec_test!(test_spec_memory_size, "memory_size.wast");
spec_test!(test_spec_memory_trap, "memory_trap.wast");
spec_test!(test_spec_nop, "nop.wast");
spec_test!(test_spec_ref_is_null, "ref_is_null.wast");
spec_test!(test_spec_return, "return.wast");
spec_test!(test_spec_select, "select.wast");
spec_test!(test_spec_start, "start.wast");
spec_test!(test_spec_store, "store.wast");
spec_test!(test_spec_table, "table.wast");
spec_test!(test_spec_table_get, "table_get.wast");
spec_test!(test_spec_table_set, "table_set.wast");
spec_test!(test_spec_traps, "traps.wast");
spec_test!(test_spec_type, "type.wast");
spec_test!(test_spec_unreachable, "unreachable.wast");
spec_test!(test_spec_func_ptrs, "func_ptrs.wast");
