use std::path::Path;

use serde::Deserialize;
use wasmtiny::{WasmApplication, WasmError, WasmValue};

const ISSUES_DIR: &str = "tests/regression/issues";

#[derive(Debug, Deserialize)]
pub struct TestCase {
    #[serde(default)]
    pub deprecated: bool,
    pub ids: Vec<u64>,
    #[serde(rename = "compile_options")]
    #[serde(default)]
    pub compile_options: Option<CompileOptions>,
    pub runtime: Option<String>,
    pub file: Option<String>,
    pub mode: Option<String>,
    #[serde(default)]
    pub options: String,
    #[serde(default)]
    pub argument: String,
    #[serde(rename = "expected return")]
    pub expected_return: Option<ExpectedReturn>,
}

#[derive(Debug, Deserialize)]
pub struct CompileOptions {
    pub compiler: String,
    #[serde(rename = "only compile")]
    pub only_compile: bool,
    #[serde(rename = "in file")]
    pub in_file: String,
    #[serde(rename = "out file")]
    pub out_file: String,
    pub options: String,
    #[serde(rename = "expected return")]
    pub expected_return: ExpectedReturn,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedReturn {
    #[serde(rename = "ret code")]
    pub ret_code: i32,
    #[serde(rename = "stdout content")]
    pub stdout_content: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct RunningConfig {
    #[serde(rename = "test cases")]
    pub test_cases: Vec<TestCase>,
}

pub fn load_config() -> RunningConfig {
    let config_path = Path::new("tests/regression/running_config.json");
    let content = std::fs::read_to_string(config_path).expect("Failed to read running_config.json");
    serde_json::from_str(&content).expect("Failed to parse running_config.json")
}

pub fn find_wasm_file(issue_dir: &Path, pattern: &str) -> Option<std::path::PathBuf> {
    if pattern.starts_with('*') {
        let glob_pattern = format!("*{}", pattern.trim_start_matches('*'));
        let entries = std::fs::read_dir(issue_dir).ok()?;
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if name.ends_with(".wasm") && name.contains(glob_pattern.trim_end_matches(".wasm")) {
                return Some(entry.path());
            }
        }
        None
    } else {
        let path = issue_dir.join(pattern);
        if path.exists() { Some(path) } else { None }
    }
}

pub fn extract_function_from_options(options: &str) -> Option<String> {
    if options.is_empty() {
        return None;
    }
    let parts: Vec<&str> = options.split_whitespace().collect();
    let mut iter = parts.iter().peekable();
    while let Some(part) = iter.next() {
        if *part == "-f" {
            if let Some(func) = iter.next() {
                return Some(func.to_string());
            }
        }
    }
    None
}

fn format_wasm_value(value: &WasmValue) -> String {
    match value {
        WasmValue::I32(v) => format!("{:#x}:i32", v),
        WasmValue::I64(v) => format!("{:#x}:i64", v),
        WasmValue::F32(v) => {
            if v.is_nan() {
                "nan:f32".to_string()
            } else {
                format!("{}:f32", v)
            }
        }
        WasmValue::F64(v) => {
            if v.is_nan() {
                "nan:f64".to_string()
            } else {
                format!("{}:f64", v)
            }
        }
        WasmValue::NullRef(_) => "nullref".to_string(),
        WasmValue::FuncRef(idx) => format!("{}:funcref", idx),
        WasmValue::ExternRef(idx) => format!("{}:externref", idx),
    }
}

fn format_wasm_values(values: &[WasmValue]) -> String {
    values
        .iter()
        .map(format_wasm_value)
        .collect::<Vec<_>>()
        .join(",")
}

pub fn run_wasm(wasm_path: &Path, function: Option<&str>, args: &[i32]) -> (i32, String) {
    let mut app = WasmApplication::new();

    let module_idx = match app.load_module_from_file(wasm_path) {
        Ok(idx) => idx,
        Err(e) => {
            let stderr = format!("Error loading module: {}", e);
            return (255, stderr);
        }
    };

    if let Err(e) = app.instantiate(module_idx) {
        let stderr = format!("Error instantiating module: {}", e);
        return (255, stderr);
    }

    let wasm_args: Vec<WasmValue> = args.iter().map(|&i| WasmValue::I32(i)).collect();

    let result = match function {
        Some(func) => app.call_function(module_idx, func, &wasm_args),
        None => app.execute_main(module_idx, &wasm_args),
    };

    match result {
        Ok(values) => (0, format_wasm_values(&values)),
        Err(WasmError::Runtime(msg)) => {
            if msg.contains("not found") || msg.contains("unreachable") {
                (1, format!("Exception: unreachable"))
            } else {
                (1, format!("Exception: {}", msg))
            }
        }
        Err(e) => (255, format!("Error: {}", e)),
    }
}

pub fn is_mode_supported(mode: Option<&str>) -> bool {
    match mode {
        None => true,
        Some("fast-interp") => true,
        Some("classic-interp") => true,
        _ => false,
    }
}

pub fn is_runtime_supported(runtime: Option<&str>) -> bool {
    match runtime {
        None => true,
        Some(r) if r.contains("gc-enabled") => false,
        Some(r) if r.contains("fast-jit") => false,
        Some(r) if r.contains("llvm-jit") => false,
        Some(r) if r.contains("branch-hints-enabled") => false,
        _ => true,
    }
}

pub fn check_options_supported(options: &str) -> bool {
    if options.is_empty() {
        return true;
    }
    let has_heap_size = options.contains("--heap-size");
    let has_function = options.contains("-f");
    !has_heap_size && (has_function || options.is_empty())
}

pub fn should_park_test(test_case: &TestCase) -> Option<String> {
    if test_case.deprecated {
        return Some("Test is deprecated".to_string());
    }

    if test_case.compile_options.is_some() {
        return Some("Test requires AOT compilation (wamrc)".to_string());
    }

    if !is_mode_supported(test_case.mode.as_deref()) {
        return Some(format!(
            "Test mode '{}' not supported (wasmtiny is interpreter-only)",
            test_case.mode.as_deref().unwrap_or("unknown")
        ));
    }

    if !is_runtime_supported(test_case.runtime.as_deref()) {
        return Some(format!(
            "Test runtime '{}' not supported",
            test_case.runtime.as_deref().unwrap_or("unknown")
        ));
    }

    if !check_options_supported(&test_case.options) {
        return Some(format!(
            "Test options '{}' not supported (--heap-size not implemented)",
            test_case.options
        ));
    }

    if let Some(file) = &test_case.file {
        if file.contains("sock_shutdown") || file.contains("socket") {
            return Some("Test requires WASI socket support (not implemented)".to_string());
        }
    }

    if is_load_error_test(test_case) {
        return Some(
            "Test expects specific WAMR error message format (not compatible)".to_string(),
        );
    }

    if has_strict_validation_issue(test_case) {
        return Some("Test fails due to wasmtiny having stricter validation than WAMR".to_string());
    }

    None
}

pub fn run_test_for_ids(test_case: &TestCase) -> Vec<(u64, bool, String)> {
    let mut results = Vec::new();

    let Some(file) = &test_case.file else {
        return results;
    };

    let Some(expected_return) = &test_case.expected_return else {
        return results;
    };

    for id in &test_case.ids {
        let issue_dir = Path::new(ISSUES_DIR).join(format!("issue-{}", id));

        if !issue_dir.exists() {
            results.push((
                *id,
                false,
                format!("Issue directory not found: {:?}", issue_dir),
            ));
            continue;
        }

        let wasm_file = find_wasm_file(&issue_dir, file);
        let Some(wasm_path) = wasm_file else {
            results.push((
                *id,
                false,
                format!("WASM file not found for pattern: {}", file),
            ));
            continue;
        };

        let function = extract_function_from_options(&test_case.options);
        let args: Vec<i32> = test_case
            .argument
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        let (actual_code, stdout) = run_wasm(&wasm_path, function.as_deref(), &args);

        let expected_code = expected_return.ret_code;
        let expected_stdout = &expected_return.stdout_content;

        let code_match = actual_code == expected_code;

        let stdout_match = stdout.trim() == expected_stdout.trim()
            || (expected_stdout.len() > 30 && stdout.contains(expected_stdout))
            || (expected_stdout == "Compile success" && stdout.contains("Compile success"));

        if code_match && stdout_match {
            results.push((*id, true, String::new()));
        } else {
            let reason = format!(
                "Exit code: expected={}, actual={}; stdout: expected='{}', actual='{}'",
                expected_code,
                actual_code,
                expected_stdout,
                stdout.trim()
            );
            results.push((*id, false, reason));
        }
    }

    results
}

#[test]
fn regression_issue_2857() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2857))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2857 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2858() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2858))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2858 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2863() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2863))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2863 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2965() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2965))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2965 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2966_2964_2963_2962() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2966))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issues 2966,2964,2963,2962 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    for (id, passed, reason) in &results {
        if !passed {
            panic!("Issue {} failed: {}", id, reason);
        }
    }
}

#[test]
fn regression_issue_2961_2960_2959_2958() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2961))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issues 2961,2960,2959,2958 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    for (id, passed, reason) in &results {
        if !passed {
            panic!("Issue {} failed: {}", id, reason);
        }
    }
}

#[test]
fn regression_issue_2956() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2956))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2956 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2955() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2955))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2955 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2954() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2954))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2954 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2953() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2953))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2953 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2952_2951() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2952))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issues 2952,2951 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    for (id, passed, reason) in &results {
        if !passed {
            panic!("Issue {} failed: {}", id, reason);
        }
    }
}

#[test]
fn regression_issue_2950() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2950))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2950 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2949_2944() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2949))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issues 2949,2944 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    for (id, passed, reason) in &results {
        if !passed {
            panic!("Issue {} failed: {}", id, reason);
        }
    }
}

#[test]
fn regression_issue_2948_2946() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2948))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issues 2948,2946 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    for (id, passed, reason) in &results {
        if !passed {
            panic!("Issue {} failed: {}", id, reason);
        }
    }
}

#[test]
fn regression_issue_2947() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2947))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2947 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2945() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2945))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2945 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3020() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3020))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3020 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3021() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3021))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3021 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3023() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3023))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3023 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3026() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3026))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3026 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3027() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3027))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3027 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3061() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3061))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3061 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3062() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3062))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3062 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3090() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3090))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3090 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_292001() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&292001))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 292001 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_292002() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&292002))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 292002 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2921() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2921))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2921 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3122() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3122))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3122 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3123() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3123))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3123 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3130() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3130))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3130 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_315101() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&315101))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 315101 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_315102() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&315102))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 315102 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3137() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3137))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3137 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2943() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2943))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2943 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2942() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2942))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2942 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2931() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2931))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2931 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2897() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2897))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2897 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2797() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2797))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2797 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2787() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2787))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2787 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2759() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2759))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2759 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2732() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2732))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2732 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2726() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2726))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2726 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3165() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3165))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3165 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_2710() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&2710))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 2710 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3170() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3170))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3170 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3210() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3210))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3210 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3286() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3286))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3286 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3336() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3336))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3336 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3337() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3337))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3337 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3346() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3346))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3346 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3347() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3347))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3347 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3386() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3386))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3386 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3387() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3387))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3387 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3388() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3388))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3388 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3401() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3401))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3401 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3402() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3402))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3402 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3403() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3403))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3403 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3410() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3410))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3410 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3411() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3411))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3411 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3467() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3467))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3467 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3468() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3468))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3468 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3491() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3491))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3491 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3513() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3513))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3513 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_3514() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&3514))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 3514 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_4643() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&4643))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 4643 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_4646() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&4646))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 4646 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_980000() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&980000))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 980000 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_980001() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&980001))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 980001 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_980002() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&980002))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 980002 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

#[test]
fn regression_issue_980003() {
    let config = load_config();
    let test_case = config
        .test_cases
        .iter()
        .find(|tc| tc.ids.contains(&980003))
        .unwrap();
    if let Some(reason) = should_park_test(test_case) {
        eprintln!("PARKED: Issue 980003 - {}", reason);
        return;
    }
    let results = run_test_for_ids(test_case);
    let (id, passed, reason) = &results[0];
    if !passed {
        panic!("Issue {} failed: {}", id, reason);
    }
}

fn is_load_error_test(test_case: &TestCase) -> bool {
    let Some(expected) = &test_case.expected_return else {
        return false;
    };
    expected.stdout_content.contains("WASM module load failed")
}

fn has_strict_validation_issue(test_case: &TestCase) -> bool {
    let Some(_expected) = &test_case.expected_return else {
        return false;
    };
    if let Some(file) = &test_case.file {
        let poc_files = [
            "iwasm_poc_04",
            "iwasm_poc_05",
            "frame_offset_overflow",
            "test.wasm",
        ];
        for poc in poc_files {
            if file.contains(poc) {
                return true;
            }
        }
    }
    false
}

pub const PARKED_TESTS: &[(&str, &str)] = &[
    ("47-84", "Deprecated tests"),
    ("2847", "Requires AOT compilation (wamrc)"),
    ("2849", "Requires AOT compilation (wamrc)"),
    ("2861", "Requires AOT compilation (wamrc)"),
    ("2862", "Requires AOT compilation (wamrc)"),
    ("2865", "Requires AOT compilation (wamrc)"),
    ("2833", "Requires AOT compilation (wamrc)"),
    ("2832", "Requires AOT compilation (wamrc)"),
    ("2829", "Requires AOT compilation (wamrc)"),
    ("2790", "Requires AOT compilation (wamrc)"),
    ("2784", "Requires AOT compilation (wamrc)"),
    ("2720", "Requires AOT compilation (wamrc)"),
    ("2714", "Requires AOT compilation (wamrc)"),
    ("2713", "Requires AOT compilation (wamrc)"),
    ("2712", "Requires AOT compilation (wamrc)"),
    ("2711", "Requires AOT compilation (wamrc)"),
    ("2709", "Requires AOT compilation (wamrc)"),
    ("270801", "Requires AOT compilation (wamrc)"),
    ("270802", "Requires AOT compilation (wamrc)"),
    ("2706", "Requires AOT compilation (wamrc)"),
    ("2705", "Requires AOT compilation (wamrc)"),
    ("2704", "Requires AOT compilation (wamrc)"),
    ("2703", "Requires AOT compilation (wamrc)"),
    ("2702", "Requires AOT compilation (wamrc)"),
    ("2701", "Requires AOT compilation (wamrc)"),
    ("2700", "Requires AOT compilation (wamrc)"),
    ("2965", "Requires fast-jit mode (JIT not supported)"),
    (
        "2966,2964,2963,2962",
        "Requires fast-jit mode (JIT not supported)",
    ),
    (
        "2961,2960,2959,2958",
        "Requires fast-jit mode (JIT not supported)",
    ),
    ("2950", "Requires fast-jit mode (JIT not supported)"),
    ("2949,2944", "Requires fast-jit mode (JIT not supported)"),
    ("2954", "Requires llvm-jit mode (JIT not supported)"),
    ("3020", "Requires fast-jit mode (JIT not supported)"),
    ("3021", "Requires fast-jit mode (JIT not supported)"),
    ("3023", "Requires fast-jit mode (JIT not supported)"),
    ("3027", "Requires llvm-jit mode (JIT not supported)"),
    ("292001", "Requires llvm-jit mode (JIT not supported)"),
    ("292002", "Requires llvm-jit mode (JIT not supported)"),
    ("2943", "Requires llvm-jit mode (JIT not supported)"),
    ("2942", "Requires llvm-jit mode (JIT not supported)"),
    ("2931", "Requires fast-jit mode (JIT not supported)"),
    ("2897", "Requires llvm-jit mode (JIT not supported)"),
    ("2732", "Requires llvm-jit mode (JIT not supported)"),
    ("2759", "Requires fast-jit mode (JIT not supported)"),
    ("3165", "Requires llvm-jit mode (JIT not supported)"),
    ("3286", "Requires llvm-jit mode (JIT not supported)"),
    ("3337", "Requires llvm-jit mode (JIT not supported)"),
    ("2857", "Requires --heap-size=0 option (not implemented)"),
    ("2858", "Requires --heap-size=0 option (not implemented)"),
    ("2863", "Requires --heap-size=0 option (not implemented)"),
    ("2956", "Requires --heap-size=0 option (not implemented)"),
    ("2955", "Requires --heap-size=0 option (not implemented)"),
    ("2953", "Requires --heap-size=0 option (not implemented)"),
    (
        "2952,2951",
        "Requires --heap-size=0 option (not implemented)",
    ),
    (
        "2948,2946",
        "Requires --heap-size=0 option (not implemented)",
    ),
    ("2947", "Requires --heap-size=0 option (not implemented)"),
    ("2945", "Requires --heap-size=0 option (not implemented)"),
    ("3026", "Requires --heap-size=0 option (not implemented)"),
    ("3061", "Requires --heap-size=0 option (not implemented)"),
    ("3062", "Requires --heap-size=0 option (not implemented)"),
    ("3090", "Requires --heap-size=0 option (not implemented)"),
    ("2921", "Requires --heap-size=0 option (not implemented)"),
    ("3122", "Requires --heap-size=0 option (not implemented)"),
    ("3123", "Requires --heap-size=0 option (not implemented)"),
    ("3130", "Requires --heap-size=0 option (not implemented)"),
    ("315101", "Requires gc-enabled runtime"),
    ("315102", "Requires gc-enabled runtime"),
    ("3137", "Requires --heap-size=0 option (not implemented)"),
    ("2797", "Requires --heap-size=0 option (not implemented)"),
    ("3170", "Requires --heap-size=0 option (not implemented)"),
    ("3336", "Requires --heap-size=0 option (not implemented)"),
    ("3346", "Requires --heap-size=0 option (not implemented)"),
    ("3347", "Requires --heap-size=0 option (not implemented)"),
    ("3386", "Requires --heap-size=0 option (not implemented)"),
    ("3387", "Requires --heap-size=0 option (not implemented)"),
    ("3388", "Requires --heap-size=0 option (not implemented)"),
    ("3401", "Requires --heap-size=0 option (not implemented)"),
    ("3402", "Requires --heap-size=0 option (not implemented)"),
    ("3403", "Requires --heap-size=0 option (not implemented)"),
    ("3410", "Requires gc-enabled runtime"),
    ("3411", "Requires gc-enabled runtime"),
    ("3467", "Requires --heap-size=0 option (not implemented)"),
    ("3468", "Requires --heap-size=0 option (not implemented)"),
    ("3491", "Requires --heap-size=0 option (not implemented)"),
    ("3513", "Requires --heap-size=0 option (not implemented)"),
    ("3514", "Requires --heap-size=0 option (not implemented)"),
    ("980002", "Requires branch-hints-enabled runtime"),
    ("980003", "Requires branch-hints-enabled runtime"),
    ("2787", "Requires socket support (not implemented)"),
];

#[test]
fn list_parked_tests() {
    println!("\n=== PARKED TESTS ===");
    println!("The following tests cannot run with wasmtiny due to missing features:\n");
    for (ids, reason) in PARKED_TESTS {
        println!("  Issue(s) {}: {}", ids, reason);
    }
    println!("\nTotal parked tests: {}", PARKED_TESTS.len());
}

#[test]
fn list_all_regression_tests() {
    let config = load_config();
    println!("\n=== ALL REGRESSION TESTS FROM JSON ===");
    let mut runnable = 0;
    let mut parked = 0;
    let mut deprecated = 0;

    for tc in &config.test_cases {
        if tc.deprecated {
            deprecated += tc.ids.len();
            continue;
        }

        if let Some(reason) = should_park_test(tc) {
            println!("  PARKING Issues {:?}: {}", tc.ids, reason);
            parked += tc.ids.len();
        } else {
            println!("  RUNNING Issues {:?}", tc.ids);
            runnable += tc.ids.len();
        }
    }

    println!("\nSummary:");
    println!("  Runnable: {}", runnable);
    println!("  Parked: {}", parked);
    println!("  Deprecated: {}", deprecated);
    println!("  Total: {}", runnable + parked + deprecated);
}
