## Why

The project currently relies on legacy WAMR test infrastructure (Python/Bash scripts, C++ googletest) that was carried over from the fork. This orchestration is already marked deprecated in the codebase (`test_wamr.sh` line 8), pointing to a non-existent `tests/integration/` directory. The test suite needs to be migrated to native Rust to match the project's direction and enable proper CI/CD integration.

## What Changes

- Create a new Rust-based spec test harness crate (`tests/spec/`)
- Vendor WebAssembly core spec test files instead of downloading at runtime
- Apply existing WAMR patches for compatibility
- Build a test runner that executes spec tests against interpreter, JIT, and AOT backends
- Remove old orchestration scripts: `test_wamr.sh`, `runtest.py`, `all.sh`, etc.
- Skip unsupported features: SIMD, threads, memory64, GC, exception-handling, WASI

## Capabilities

### New Capabilities
- `spec-test-harness`: Rust crate that runs WebAssembly spec test suite against all backends
- `spec-test-fixtures`: Vendored spec test files (wast/wasm) with WAMR compatibility patches

### Modified Capabilities
(none - this is a new testing capability)

## Impact

- New test crate: `tests/spec/`
- Dependencies: `wabt` crate for wat2wasm conversion
- Removed: 9 legacy orchestration files in `tests/wamr-test-suites/`
- Spec test patches from `tests/wamr-test-suites/spec-test-script/*.patch` will be applied to fixtures
