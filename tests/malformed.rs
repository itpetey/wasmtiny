use walkdir::WalkDir;
use wasmtiny::WasmApplication;

#[test]
fn malformed_github_tests() {
    let test_cases: Vec<_> = WalkDir::new("tests/malformed")
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    assert!(!test_cases.is_empty(), "No malformed test cases found");

    let mut app = WasmApplication::new();

    for entry in test_cases {
        let result = app.load_module_from_file(entry.path());
        assert!(
            result.is_err(),
            "Expected failure for malformed file: {}",
            entry.path().display()
        );
    }
}
