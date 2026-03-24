use walkdir::WalkDir;

#[test]
fn regression_issues_exist() {
    let issue_dirs: Vec<_> = WalkDir::new("tests/regression")
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("issue-"))
                .unwrap_or(false)
        })
        .collect();
}
