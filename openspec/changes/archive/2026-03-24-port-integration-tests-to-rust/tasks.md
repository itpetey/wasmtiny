## 1. Project Setup

- [x] 1.1 Create `tests/integration/` directory structure
- [x] 1.2 Add `tests/integration/Cargo.toml` with workspace configuration
- [x] 1.3 Add `tests/integration/src/main.rs` CLI entry point
- [x] 1.4 Add `tests/integration/src/lib.rs` for test harness
- [x] 1.5 Configure dependencies (clap, anyhow, tempfile, walkdir)

## 2. Test Utilities Library

- [x] 2.1 Create `tests/integration/wamr-test-utils/` crate
- [x] 2.2 Implement binary path discovery with `WAMR_BIN_PATH` support
- [x] 2.3 Implement `WamrProcess` wrapper with timeout support
- [x] 2.4 Implement temp directory management helpers
- [x] 2.5 Add WASM test fixture path resolution

## 3. CLI Tool Implementation

- [x] 3.1 Define CLI arguments with clap (--suite, --mode, --help)
- [x] 3.2 Implement test suite selection logic
- [x] 3.3 Implement runtime mode selection logic
- [x] 3.4 Implement proper exit codes (0, 1, 2)
- [x] 3.5 Add version and help output

## 4. Spec Test Module

- [x] 4.1 Create `tests/integration/src/spec.rs` module
- [x] 4.2 Map spec test cases from `tests/wamr-test-suites/spec-test-script/`
- [x] 4.3 Implement test case execution with result capture
- [x] 4.4 Add assertions for expected output matching
- [x] 4.5 Test with interpreter mode

## 5. Standalone Test Module

- [x] 5.1 Create `tests/integration/src/standalone.rs` module
- [x] 5.2 Map standalone tests from `tests/standalone/`
- [x] 5.3 Implement shell script execution wrapper
- [x] 5.4 Add temp directory isolation per test
- [x] 5.5 Verify test pass/fail matching original behavior

## 6. Malformed Test Module

- [x] 6.1 Create `tests/integration/src/malformed.rs` module
- [x] 6.2 Map malformed test cases from `tests/malformed/`
- [x] 6.3 Implement negative test assertions (expect failure)
- [x] 6.4 Verify error messages match expected patterns

## 7. Regression Test Module

- [x] 7.1 Create `tests/integration/src/regression.rs` module
- [x] 7.2 Map regression tests from `tests/regression/ba-issues/`
- [x] 7.3 Implement regression test execution
- [x] 7.4 Add pass/fail matching with original test expectations

## 8. CI Integration

- [x] 8.1 Update `.github/workflows/nightly_run.yml` to run Rust tests
- [x] 8.2 Ensure WAMR binaries are built before test execution
- [x] 8.3 Add test result artifact collection
- [x] 8.4 Verify tests pass in CI environment

## 9. Documentation

- [x] 9.1 Add `tests/integration/README.md` with usage instructions
- [x] 9.2 Document `WAMR_BIN_PATH` and `WAMR_MODE` environment variables
- [x] 9.3 Add examples for running specific test suites
- [x] 9.4 Add deprecation notices to original Python/shell scripts
