## Context

The project has legacy test infrastructure from the WAMR fork that is:
- Already marked deprecated (`test_wamr.sh` line 8: "This script is deprecated in favor of the Rust-based integration tests")
- Uses Python/Bash orchestration (9 scripts in `tests/wamr-test-suites/`)
- Downloads WebAssembly spec repos at runtime instead of vendoring
- Has no clear integration with Cargo test ecosystem

The goal is to replace this with native Rust tests that:
- Run as standard `cargo test` 
- Test all three backends (interpreter, JIT, AOT)
- Are maintainable and CI-friendly

## Goals / Non-Goals

**Goals:**
- Create Rust spec test harness using `wabt` crate for wat2wasm
- Vendor spec test fixtures (don't download at runtime)
- Apply WAMR patches from `spec-test-script/*.patch` to fixtures
- Run spec tests against interpreter, JIT, and AOT backends
- Remove all legacy orchestration scripts

**Non-Goals:**
- Implement SIMD, threads, memory64, GC, exception-handling, or WASI (not supported by wasmtiny)
- Migrate C++ unit tests (separate change)
- Implement multi-threaded test execution (can add later)

## Decisions

### 1. Use wabt Rust crate vs wat2wasm binary

**Decision:** Use `wabt` crate (`wabt::wat2wasm`)

**Rationale:**
- No external binary dependency
- Faster test execution (no subprocess)
- Matches official spec tools
- Minor behavioral differences from WAMR's wat2wasm are valuable for robustness testing

**Alternative considered:** Use wat2wasm binary
- Requires WAMR_BIN_PATH or build system integration
- More complex CI/CD
- Would match WAMR exactly (arguable advantage)

### 2. Vendor spec files vs download at runtime

**Decision:** Vendor in `tests/spec/fixtures/`

**Rationale:**
- Deterministic builds
- Offline CI support
- Can apply patches once
- Faster test execution

**Alternative considered:** Download at runtime
- Matches current WAMR approach
- Always gets latest spec
- But: network dependency, non-deterministic

### 3. Skip unsupported features

**Decision:** Filter out spec tests for features not supported by wasmtiny

**Features to skip:**
- `simd/` - no v128 support
- `threads/` - no atomic memory instructions
- `memory64/` - no 64-bit memory
- `gc/` - no array/struct types
- `exception-handling/` - no try/catch
- `wasi/` - deliberately not implemented

**Rationale:** Running unsupported tests wastes CI time and creates false failures

### 4. Test harness architecture

```
tests/spec/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Test runner, backend abstraction
│   ├── wat2wasm.rs     # wabt wrapper
│   ├── runner.rs       # Test execution logic
│   └── fixtures/       # Vendored spec files
│       ├── core/       # wasm-core-1.x spec tests
│       └── patches/    # Applied .patch files
```

Each test file (e.g., `test.wast`) becomes a Rust test that:
1. Uses `wat2wasm` to convert to binary
2. Runs through interpreter → asserts expected behavior
3. Runs through JIT → asserts same behavior
4. Runs through AOT → asserts same behavior

## Risks / Trade-offs

- **[Risk]** wabt crate behavior differs from WAMR's wat2wasm
  - **Mitigation:** This tests robustness; document known differences
  - May need to skip specific test cases that fail due to tooling differences

- **[Risk]** Maintaining patches is ongoing work
  - **Mitigation:** Automate patch application in build script
  - Track which patches are applied in a manifest

- **[Risk]** Spec test count (~50 files) may slow down CI
  - **Mitigation:** Can add parallel execution later
  - Start with serial, optimize if needed

- **[Risk]** Some spec tests may rely on WASI
  - **Mitigation:** Filter out wasi-related tests explicitly

## Migration Plan

1. Create `tests/spec/` crate structure
2. Download and vendor spec test files
3. Apply WAMR patches from `spec-test-script/`
4. Implement wat2wasm wrapper using wabt crate
5. Create test runner with backend abstraction
6. Run tests, fix failures, skip unsupported features
7. Remove legacy orchestration scripts

## Open Questions

- Should spec tests run in parallel (backends in parallel, or test files in parallel)?
- How to handle test failures - fail all backends if any fails, or per-backend reporting?
- Should we include the full spec test suite or a representative subset for CI speed?
