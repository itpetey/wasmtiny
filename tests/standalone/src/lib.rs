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

pub fn discover_binary(name: &str) -> anyhow::Result<std::path::PathBuf> {
    if let Ok(path) = std::env::var("WAMR_BIN_PATH") {
        let binary = std::path::PathBuf::from(path).join(name);
        if binary.exists() {
            return Ok(binary);
        }
    }

    let root = project_root();
    let candidates = vec![
        root.join("build"),
        root.join("build/lite_mode"),
        root.join("build/minimal"),
    ];

    for candidate in candidates {
        let binary = candidate.join(name);
        if binary.exists() {
            return Ok(binary);
        }
    }

    let search_paths = vec!["/usr/local/bin", "/usr/bin"];

    for search_path in search_paths {
        let binary = std::path::PathBuf::from(search_path).join(name);
        if binary.exists() {
            return Ok(binary);
        }
    }

    anyhow::bail!(
        "WAMR binary '{}' not found. Set WAMR_BIN_PATH environment variable.",
        name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_simple() {
        let iwasm = match discover_binary("iwasm") {
            Ok(p) => p,
            Err(_) => {
                eprintln!("SKIP: iwasm binary not found - set WAMR_BIN_PATH");
                return;
            }
        };

        let root = project_root();
        let wasm = root.join("tests/standalone/simple/simple.wasm");
        if !wasm.exists() {
            eprintln!("SKIP: tests/standalone/simple/simple.wasm not found");
            return;
        }

        let result = std::process::Command::new(&iwasm).arg(&wasm).output();
        assert!(result.is_ok(), "Failed to run iwasm");
        assert!(
            result.unwrap().status.success(),
            "simple.wasm should execute"
        );
    }

    #[test]
    fn discover_all_standalone_tests() {
        use std::collections::HashMap;
        use std::fs;

        let root = project_root();
        let standalone_dir = root.join("tests/standalone");

        if !standalone_dir.exists() {
            eprintln!("SKIP: tests/standalone not found");
            return;
        }

        let mut tests: HashMap<String, Vec<std::path::PathBuf>> = HashMap::new();

        for entry in fs::read_dir(standalone_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut test_files = Vec::new();

                if path.join("run.sh").exists() {
                    test_files.push(path.join("run.sh"));
                }

                for ext in &["wasm", "wat"] {
                    if let Ok(entries) = fs::read_dir(&path) {
                        for e in entries.flatten() {
                            if e.path().extension().and_then(|s| s.to_str()) == Some(ext) {
                                test_files.push(e.path());
                            }
                        }
                    }
                }

                if !test_files.is_empty() {
                    tests.insert(name, test_files);
                }
            }
        }

        assert!(!tests.is_empty(), "No standalone tests discovered");

        let run_shell_count = tests
            .values()
            .flatten()
            .filter(|p| p.file_name().and_then(|s| s.to_str()) == Some("run.sh"))
            .count();
        assert!(run_shell_count > 0, "No run.sh scripts found");
    }
}
