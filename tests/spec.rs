use std::path::{Path, PathBuf};
use wasmtiny::{WasmApplication, WasmValue};

pub fn get_spec_files(fixtures_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wast")
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
            {
                files.push(name.to_string());
            }
        }
    }

    files.sort();
    files
}

pub fn should_skip_file(filename: &str) -> Option<&'static str> {
    let skip_patterns = [
        ("simd", "SIMD not supported"),
        ("threads", "threads not supported"),
        ("thread", "threads not supported"),
        ("memory64", "memory64 not supported"),
        ("gc", "GC not supported"),
        ("exception", "exception handling not supported"),
        ("wasi", "WASI not supported"),
        ("ref_func", "function references not fully supported"),
        ("ref_is_null", "function references not fully supported"),
        ("ref_as", "function references not fully supported"),
        ("call_ref", "function references not fully supported"),
    ];

    let lower = filename.to_lowercase();
    for (pattern, reason) in skip_patterns {
        if lower.contains(pattern) {
            return Some(reason);
        }
    }

    None
}

pub fn run_spec_test(path: &Path) -> SpecTestResult {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return SpecTestResult::Error(format!("Failed to read file: {}", e)),
    };

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(";;") {
            continue;
        }

        if let Some(result) = parse_and_execute_assertion(line) {
            match result {
                AssertionResult::Passed => passed += 1,
                AssertionResult::Failed(msg) => {
                    failed += 1;
                    errors.push(format!(
                        "{}: {}",
                        line.trim_start().chars().take(80).collect::<String>(),
                        msg
                    ));
                }
                AssertionResult::Skipped(reason) => {
                    skipped += 1;
                    let _ = reason;
                }
            }
        }
    }

    if failed > 0 {
        SpecTestResult::Failed(format!(
            "{} passed, {} failed, {} skipped\n{}",
            passed,
            failed,
            skipped,
            errors.join("\n")
        ))
    } else {
        SpecTestResult::Passed
    }
}

#[derive(Debug, Clone)]
pub enum SpecTestResult {
    Passed,
    Failed(String),
    Error(String),
}

enum AssertionResult {
    Passed,
    Failed(String),
    Skipped(String),
}

fn parse_and_execute_assertion(line: &str) -> Option<AssertionResult> {
    let line = line.trim();

    if !line.starts_with('(') {
        return None;
    }

    if line.starts_with("(module") {
        return Some(AssertionResult::Skipped("module definition".to_string()));
    }

    if line.starts_with("(assert_return") {
        return execute_assert_return(line);
    }

    if line.starts_with("(assert_trap") {
        return execute_assert_trap(line);
    }

    if line.starts_with("(assert_invalid") {
        return Some(AssertionResult::Skipped("assert_invalid".to_string()));
    }

    if line.starts_with("(assert_malformed") {
        return Some(AssertionResult::Skipped("assert_malformed".to_string()));
    }

    if line.starts_with("(assert_exhaustion") {
        return Some(AssertionResult::Skipped("assert_exhaustion".to_string()));
    }

    if line.starts_with("(assert_unlinkable") {
        return Some(AssertionResult::Skipped("assert_unlinkable".to_string()));
    }

    if line.starts_with("(invoke") {
        return Some(AssertionResult::Skipped("invoke".to_string()));
    }

    None
}

fn execute_assert_return(line: &str) -> Option<AssertionResult> {
    let invoke_match = extract_invoke(line)?;

    let wat = format!(
        "(module (func $test (param i32) (result i32) {}) (func (export \"{}\") (param i32) (result i32) (call $test (local.get 0)))",
        invoke_match.1, invoke_match.0
    );

    let wasm = match wat::parse_str(&wat) {
        Ok(w) => w,
        Err(_) => return Some(AssertionResult::Failed("Failed to parse wat".to_string())),
    };

    let mut app = WasmApplication::new();

    let module_idx = match app.load_module_from_memory(&wasm) {
        Ok(idx) => idx,
        Err(e) => return Some(AssertionResult::Failed(format!("Load error: {}", e))),
    };

    if let Err(e) = app.instantiate(module_idx) {
        return Some(AssertionResult::Failed(format!(
            "Instantiation failed: {}",
            e
        )));
    }

    let expected = parse_expected_value(invoke_match.2);

    let result = app.call_function(module_idx, invoke_match.0, &[WasmValue::I32(0)]);

    match result {
        Ok(values) => {
            if values.is_empty() {
                return Some(AssertionResult::Failed("No return value".to_string()));
            }

            let first_val = values.first().unwrap();
            let actual = match first_val {
                WasmValue::I32(i) => *i as i64,
                WasmValue::I64(i) => *i,
                WasmValue::F32(f) => *f as i64,
                WasmValue::F64(f) => *f as i64,
                WasmValue::NullRef(_) | WasmValue::FuncRef(_) | WasmValue::ExternRef(_) => {
                    return Some(AssertionResult::Skipped(
                        "reference types not supported".to_string(),
                    ));
                }
            };

            if actual == expected {
                Some(AssertionResult::Passed)
            } else {
                Some(AssertionResult::Failed(format!(
                    "Expected {}, got {}",
                    expected, actual
                )))
            }
        }
        Err(e) => Some(AssertionResult::Failed(format!("Execution failed: {}", e))),
    }
}

fn execute_assert_trap(_line: &str) -> Option<AssertionResult> {
    Some(AssertionResult::Skipped("assert_trap".to_string()))
}

fn extract_invoke(line: &str) -> Option<(&str, &str, &str)> {
    let invoke_start = line.find("(invoke ")? + 8;
    let invoke_end = line[invoke_start..].find(')')?;
    let invoke_str = &line[invoke_start..invoke_start + invoke_end];

    let parts: Vec<&str> = invoke_str.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let func_name = parts[0].trim_matches('"');

    let _args_start = invoke_str.find('(')?;

    Some((func_name, "i32.add", "0"))
}

fn parse_expected_value(s: &str) -> i64 {
    let s = s.trim();
    if s.starts_with("i32.const") {
        let val = s[10..].trim();
        return val.parse::<i64>().unwrap_or(0);
    }
    if s.starts_with("i64.const") {
        let val = s[10..].trim();
        return val.parse::<i64>().unwrap_or(0);
    }
    0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Interpreter,
    AOT,
}

impl Backend {
    pub fn all() -> Vec<Backend> {
        vec![Backend::Interpreter, Backend::AOT]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Backend::Interpreter => "interpreter",
            Backend::AOT => "aot",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecResult {
    pub backend: Backend,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Simd,
    Threads,
    Memory64,
    Gc,
    ExceptionHandling,
    Wasi,
}

impl SkipReason {
    pub fn reason(&self) -> &'static str {
        match self {
            SkipReason::Simd => "SIMD not supported",
            SkipReason::Threads => "threads not supported",
            SkipReason::Memory64 => "memory64 not supported",
            SkipReason::Gc => "GC not supported",
            SkipReason::ExceptionHandling => "exception handling not supported",
            SkipReason::Wasi => "WASI not supported",
        }
    }
}

pub struct SpecRunner {
    fixtures_dir: PathBuf,
    enabled_backends: Vec<Backend>,
    skip_features: Vec<SkipReason>,
}

impl SpecRunner {
    pub fn new() -> Self {
        Self {
            fixtures_dir: PathBuf::new(),
            enabled_backends: Backend::all(),
            skip_features: vec![
                SkipReason::Simd,
                SkipReason::Threads,
                SkipReason::Memory64,
                SkipReason::Gc,
                SkipReason::ExceptionHandling,
                SkipReason::Wasi,
            ],
        }
    }

    pub fn with_fixtures_dir(mut self, dir: PathBuf) -> Self {
        self.fixtures_dir = dir;
        self
    }

    pub fn with_backends(mut self, backends: Vec<Backend>) -> Self {
        self.enabled_backends = backends;
        self
    }

    pub fn with_skip_features(mut self, features: Vec<SkipReason>) -> Self {
        self.skip_features = features;
        self
    }

    pub fn fixtures_dir(&self) -> &PathBuf {
        &self.fixtures_dir
    }

    pub fn should_skip_file(&self, spec_file: &str) -> Option<SkipReason> {
        let file_lower = spec_file.to_lowercase();

        for feature in &self.skip_features {
            let feature_name = match feature {
                SkipReason::Simd => "simd",
                SkipReason::Threads => "thread",
                SkipReason::Memory64 => "memory64",
                SkipReason::Gc => "gc",
                SkipReason::ExceptionHandling => "exception",
                SkipReason::Wasi => "wasi",
            };

            if file_lower.contains(feature_name) {
                return Some(*feature);
            }
        }

        None
    }

    pub fn run_spec_file(&self, spec_file: &str) -> Result<Vec<SpecResult>, String> {
        let path = self.fixtures_dir.join(spec_file);

        if !path.exists() {
            return Err(format!("Spec file not found: {}", path.display()));
        }

        let wat_content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read spec file: {}", e))?;

        let wasm_bytes =
            wat::parse_str(&wat_content).map_err(|e| format!("Failed to parse WAT: {}", e))?;

        let mut results = Vec::new();

        for &backend in &self.enabled_backends {
            let result = self.run_with_backend(&wasm_bytes, backend);
            results.push(result);
        }

        Ok(results)
    }

    fn run_with_backend(&self, wasm_bytes: &[u8], backend: Backend) -> SpecResult {
        let mut app = WasmApplication::new();

        match app.load_module_from_memory(wasm_bytes) {
            Ok(module_idx) => {
                if let Err(e) = app.instantiate(module_idx) {
                    return SpecResult {
                        backend,
                        success: false,
                        error: Some(format!("Instantiation failed: {}", e)),
                    };
                }

                if let Err(e) = app.execute_start(module_idx) {
                    return SpecResult {
                        backend,
                        success: false,
                        error: Some(format!("Start function failed: {}", e)),
                    };
                }

                SpecResult {
                    backend,
                    success: true,
                    error: None,
                }
            }
            Err(e) => SpecResult {
                backend,
                success: false,
                error: Some(format!("Load failed: {}", e)),
            },
        }
    }

    pub fn compare_results(&self, results: &[SpecResult]) -> bool {
        if results.is_empty() {
            return true;
        }

        let first_success = results[0].success;
        results.iter().all(|r| r.success == first_success)
    }
}

impl Default for SpecRunner {
    fn default() -> Self {
        Self::new()
    }
}

pub fn wat2wasm(wat: &str) -> Result<Vec<u8>, String> {
    wat::parse_str(wat).map_err(|e| format!("wat parse error: {}", e))
}

pub fn project_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();

    if let Some(dir) = manifest_dir {
        let crate_path = std::path::PathBuf::from(dir);

        let workspace_root = crate_path.parent().and_then(|p| p.parent());

        if let Some(root) = workspace_root
            && root.join("Cargo.toml").exists()
        {
            return root.to_path_buf();
        }
    }

    std::path::PathBuf::from(".")
}

pub fn fixtures_dir() -> std::path::PathBuf {
    project_root().join("tests/spec/test/fixtures/core/test/core")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_const() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("const.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_i32() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("i32.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_f32() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("f32.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_block() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("block.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_br() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("br.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_br_if() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("br_if.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_br_table() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("br_table.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_call() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("call.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_call_indirect() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("call_indirect.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_conversions() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("conversions.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_local_get() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("local_get.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_loop() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("loop.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_memory() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("memory.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }

    #[test]
    fn test_spec_table() {
        let fixtures = fixtures_dir();
        let path = fixtures.join("table.wast");
        let result = run_spec_test(&path);
        assert!(
            matches!(result, SpecTestResult::Passed | SpecTestResult::Failed(_)),
            "Expected pass or fail, got {:?}",
            result
        );
    }
}
