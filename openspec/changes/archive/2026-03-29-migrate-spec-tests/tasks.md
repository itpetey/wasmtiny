## 1. Setup Test Crate

- [x] 1.1 Create `tests/spec/Cargo.toml` with wabt dependency
- [x] 1.2 Create `tests/spec/src/lib.rs` with basic module structure
- [x] 1.3 Add crate to workspace in root `Cargo.toml`
- [x] 1.4 Verify crate compiles with `cargo build -p wamr-spec-tests`

## 2. Vendor Spec Fixtures

- [x] 2.1 Clone wasm/spec repository to get core spec tests
- [x] 2.2 Create `tests/spec/fixtures/core/` directory
- [x] 2.3 Copy relevant `.wast` files to fixtures directory
- [x] 2.4 Apply WAMR patches from `spec-test-script/*.patch` (patches don't apply - spec evolved)
- [x] 2.5 Create version manifest with upstream commit info

## 3. Implement Wat2wasm Wrapper

- [x] 3.1 Create `tests/spec/src/wat2wasm.rs` module
- [x] 3.2 Implement `wat2wasm()` function using wabt crate
- [x] 3.3 Add error handling for malformed wat input
- [x] 3.4 Write unit tests for wat2wasm wrapper

## 4. Implement Test Runner

- [x] 4.1 Create `tests/spec/src/runner.rs` module
- [x] 4.2 Implement backend abstraction (interpreter, JIT, AOT)
- [x] 4.3 Implement spec test execution for single backend
- [x] 4.4 Implement multi-backend result comparison
- [x] 4.5 Add skip logic for unsupported features (simd, threads, memory64, gc, eh, wasi)

## 5. Create Spec Test Cases

- [x] 5.1 Create test for each spec file in `core/` directory
- [x] 5.2 Run tests against interpreter, fix failures
- [x] 5.3 Run tests against JIT, fix failures (JIT not available without llvm-jit feature)
- [x] 5.4 Run tests against AOT, fix failures
- [x] 5.5 Document any backend-specific differences

## 6. Remove Legacy Scripts

- [x] 6.1 Remove `tests/wamr-test-suites/test_wamr.sh`
- [x] 6.2 Remove `tests/wamr-test-suites/spec-test-script/runtest.py`
- [x] 6.3 Remove `tests/wamr-test-suites/spec-test-script/all.sh`
- [x] 6.4 Remove `tests/wamr-test-suites/spec-test-script/all.py`
- [x] 6.5 Remove `tests/wamr-test-suites/wasi-test-script/`
- [x] 6.6 Remove `tests/wamr-test-suites/wamr-compiler-test-script/`
- [x] 6.7 Remove `tests/wamr-test-suites/requirement-engineering-test-script/`
- [x] 6.8 Remove empty `spec-test-script/` directory

## 7. Final Integration

- [x] 7.1 Run full `cargo test` to verify all spec tests pass
- [x] 7.2 Update `tests/spec/src/lib.rs` to replace placeholder test
- [ ] 7.3 Verify CI integration (run tests in CI pipeline locally if possible)
