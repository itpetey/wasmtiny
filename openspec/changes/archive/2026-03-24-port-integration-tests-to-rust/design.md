## Context

The wasm-micro-runtime project maintains integration tests across multiple languages and formats:
- `test_wamr.sh` (1254 lines) - Main Python/Shell orchestration
- `tests/standalone/` - Individual shell script runners
- `tests/regression/ba-issues/` - Python test runner
- `tests/malformed/` - Python malformed test suite

This fragmentation creates maintenance burden, inconsistent error handling, and limits test reuse across suites.

## Goals / Non-Goals

**Goals:**
- Create a unified Rust test suite that replaces Python/shell orchestration
- Maintain parity with existing test coverage (spec, standalone, malformed, regression)
- Provide a CLI tool for flexible test execution
- Improve test output readability and debugging

**Non-Goals:**
- Porting C++ GoogleTest unit tests (separate effort)
- Modifying WAMR runtime behavior or APIs
- Replacing the CTest infrastructure for compiled unit tests
- Full property-based testing (future enhancement)

## Decisions

### 1. Rust Workspace Structure

**Decision:** Create `tests/integration/` as a standalone Cargo workspace member.

**Rationale:** Keeps integration tests separate from `src/tests/` which contains simple smoke tests. Allows independent versioning and CI configuration.

**Alternative:** Add to existing `wasmtiny` workspace. Rejected - different lifecycle and dependencies.

### 2. Test Execution Model

**Decision:** Tests run against pre-built WAMR binaries (iwasm, wamrc) via `std::process::Command`.

**Rationale:** WAMR is compiled via CMake separately. Rust tests orchestrate the pre-built artifacts rather than rebuilding. Matches existing `test_wamr.sh` model.

**Alternative:** Build WAMR from Rust. Rejected - CMake complexity and circular dependency issues.

### 3. CLI Framework

**Decision:** Use `clap` for CLI argument parsing.

**Rationale:** Standard Rust CLI choice, supports subcommands, generates help docs, widely used in Rust ecosystem.

**Alternative:** Custom argument parsing. Rejected - reinventing wheel.

### 4. Test Utilities Library

**Decision:** Create internal `wamr-test-utils` crate within workspace.

**Rationale:** Shared utilities (binary path resolution, temp directories, WASM file loading) avoid duplication across test modules.

**Contents:**
- Binary discovery (look for `iwasm`/`wamrc` in common paths)
- Temp directory management
- WAMR process execution wrapper with timeout
- Result assertion helpers

### 5. Test Organization

**Decision:** Mirror existing test categories as Rust modules:
- `spec/` - WASM spec tests
- `standalone/` - Standalone scenario tests  
- `malformed/` - Invalid input tests
- `regression/` - Regression test cases

**Rationale:** Familiar structure, clear mapping to existing tests, easier migration.

### 6. Feature Flags for Test Modes

**Decision:** Use Rust feature flags for runtime modes (interp, fast-interp, jit, aot).

**Rationale:** Allows running subset of tests per mode. Matches existing test_wamr.sh `-m` option.

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Test execution time increases | CI duration | Parallel execution with `cargo test --test-threads=N` |
| Binary path detection fails on some platforms | Tests fail to run | Configurable `WAMR_BIN_PATH` env var override |
| Feature parity with shell scripts takes multiple iterations | Gaps in coverage | Incremental port, keep originals during transition |
| Timeout handling differs from shell `timeout` | Flaky tests | Use `std::process::Command` with `std::time::Duration` |

## Open Questions

1. **CI Binary Delivery**: How do we ensure `iwasm`/`wamrc` binaries exist in CI before tests run? Options: build in same job, download from artifact, use container with pre-built binaries.

2. **Test Data Location**: Should WASM test fixtures live in `tests/integration/fixtures/` or reference existing `tests/wamr-test-suites/` directories?

3. **Parallel Test Safety**: Some tests may share state. Do we need test isolation via unique temp directories per test, or is serial execution acceptable for specific suites?

4. **Deprecation Timeline**: How long should original Python/shell scripts be retained? Suggest: 2 release cycles with deprecation warnings.
