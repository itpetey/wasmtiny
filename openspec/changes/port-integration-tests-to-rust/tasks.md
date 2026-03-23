## 1. Project Setup

- [ ] 1.1 Create `tests/integration/` directory structure
- [ ] 1.2 Add `tests/integration/Cargo.toml` with workspace configuration
- [ ] 1.3 Add `tests/integration/src/main.rs` CLI entry point
- [ ] 1.4 Add `tests/integration/src/lib.rs` for test harness
- [ ] 1.5 Configure dependencies (clap, anyhow, tempfile, walkdir)

## 2. Test Utilities Library

- [ ] 2.1 Create `tests/integration/wamr-test-utils/` crate
- [ ] 2.2 Implement binary path discovery with `WAMR_BIN_PATH` support
- [ ] 2.3 Implement `WamrProcess` wrapper with timeout support
- [ ] 2.4 Implement temp directory management helpers
- [ ] 2.5 Add WASM test fixture path resolution

## 3. CLI Tool Implementation

- [ ] 3.1 Define CLI arguments with clap (--suite, --mode, --help)
- [ ] 3.2 Implement test suite selection logic
- [ ] 3.3 Implement runtime mode selection logic
- [ ] 3.4 Implement proper exit codes (0, 1, 2)
- [ ] 3.5 Add version and help output

## 4. Spec Test Module

- [ ] 4.1 Create `tests/integration/src/spec.rs` module
- [ ] 4.2 Map spec test cases from `tests/wamr-test-suites/spec-test-script/`
- [ ] 4.3 Implement test case execution with result capture
- [ ] 4.4 Add assertions for expected output matching
- [ ] 4.5 Test with interpreter mode

## 5. Standalone Test Module

- [ ] 5.1 Create `tests/integration/src/standalone.rs` module
- [ ] 5.2 Map standalone tests from `tests/standalone/`
- [ ] 5.3 Implement shell script execution wrapper
- [ ] 5.4 Add temp directory isolation per test
- [ ] 5.5 Verify test pass/fail matching original behavior

## 6. Malformed Test Module

- [ ] 6.1 Create `tests/integration/src/malformed.rs` module
- [ ] 6.2 Map malformed test cases from `tests/malformed/`
- [ ] 6.3 Implement negative test assertions (expect failure)
- [ ] 6.4 Verify error messages match expected patterns

## 7. Regression Test Module

- [ ] 7.1 Create `tests/integration/src/regression.rs` module
- [ ] 7.2 Map regression tests from `tests/regression/ba-issues/`
- [ ] 7.3 Implement regression test execution
- [ ] 7.4 Add pass/fail matching with original test expectations

## 8. CI Integration

- [ ] 8.1 Update `.github/workflows/nightly_run.yml` to run Rust tests
- [ ] 8.2 Ensure WAMR binaries are built before test execution
- [ ] 8.3 Add test result artifact collection
- [ ] 8.4 Verify tests pass in CI environment

## 9. Documentation

- [ ] 9.1 Add `tests/integration/README.md` with usage instructions
- [ ] 9.2 Document `WAMR_BIN_PATH` and `WAMR_MODE` environment variables
- [ ] 9.3 Add examples for running specific test suites
- [ ] 9.4 Add deprecation notices to original Python/shell scripts
