## Why

The wasm-micro-runtime project currently uses fragmented test infrastructure across multiple languages (Python scripts, Shell scripts, C++ with GoogleTest). This makes tests harder to maintain, limits code sharing, and creates friction for contributors familiar with only one language. Porting integration tests to Rust would unify the test codebase, enable use of Rust's modern testing ecosystem (criterion for benchmarks, proptest for property testing), and improve CI/CD reliability.

## What Changes

- Create a new Rust-based integration test suite under `tests/integration/`
- Port the main `test_wamr.sh` orchestration logic to Rust
- Port `tests/standalone/` shell script tests to Rust using `std::process::Command`
- Create Rust test utilities library for common WAMR testing operations
- Add Cargo workspace configuration for the test suite
- Update CI workflows to run Rust-based integration tests
- Deprecate (but retain) original Python/shell test scripts during transition period

## Capabilities

### New Capabilities

- `rust-integration-test-framework`: Unified Rust framework providing test utilities, WAMR binary management, and result assertion helpers
- `integration-test-port`: Ported integration tests covering spec, standalone, malformed, and regression test suites
- `test-cli-tool`: Command-line tool for running integration tests with configurable options (test suites, modes, features)

### Modified Capabilities

- None (this is a refactoring/porting effort, not a behavior change)

## Impact

- **New Files**: `tests/integration/**/*.rs`, `tests/integration/Cargo.toml`, `tests/integration/README.md`
- **Modified Files**: `.github/workflows/*.yml` (CI updates), `tests/wamr-test-suites/test_wamr.sh` (deprecation notices)
- **Dependencies**: New Rust crates - `anyhow`, `clap` (CLI), `tempfile`, `walkdir`
- **Removed (after transition)**: Original Python test runners, shell script tests (with deprecation warning period)
