pub fn project_root() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();

    if let Some(dir) = manifest_dir {
        let crate_path = std::path::PathBuf::from(dir);

        let workspace_root = crate_path.parent().and_then(|p| p.parent());

        if let Some(root) = workspace_root {
            if root.join("Cargo.toml").exists() {
                return root.to_path_buf();
            }
        }
    }

    std::path::PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_tests_exist() {
        let root = project_root();
        let spec_dir = root.join("tests/wamr-test-suites/spec-test-script");

        if !spec_dir.exists() {
            eprintln!("SKIP: tests/wamr-test-suites/spec-test-script not found");
            return;
        }

        assert!(
            spec_dir.join("runtest.py").exists(),
            "spec test runner not found"
        );
    }

    #[test]
    fn spec_test_runner_exists() {
        let root = project_root();
        let runner = root.join("tests/wamr-test-suites/spec-test-script/runtest.py");
        assert!(runner.exists(), "spec test runner runtest.py not found");
    }
}
