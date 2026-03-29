## 1. Setup Test Crate

- [ ] 1.1 Create `tests/spec/Cargo.toml` with wabt dependency
- [ ] 1.2 Create `tests/spec/src/lib.rs` with basic module structure
- [ ] 1.3 Add crate to workspace in root `Cargo.toml`
- [ ] 1.4 Verify crate compiles with `cargo build -p wamr-spec-tests`

## 2. Vendor Spec Fixtures

- [ ] 2.1 Clone wasm/spec repository to get core spec tests
- [ ] 2.2 Create `tests/spec/fixtures/core/` directory
- [ ] 2.3 Copy relevant `.wast` files to fixtures directory
- [ ] 2.4 Apply WAMR patches from `spec-test-script/*.patch`
- [ ] 2.5 Create version manifest with upstream commit info

## 3. Implement Wat2wasm Wrapper

- [ ] 3.1 Create `tests/spec/src/wat2wasm.rs` module
- [ ] 3.2 Implement `wat2wasm()` function using wabt crate
- [ ] 3.3 Add error handling for malformed wat input
- [ ] 3.4 Write unit tests for wat2wasm wrapper

## 4. Implement Test Runner

- [ ] 4.1 Create `tests/spec/src/runner.rs` module
- [ ] 4.2 Implement backend abstraction (interpreter, JIT, AOT)
- [ ] 4.3 Implement spec test execution for single backend
- [ ] 4.4 Implement multi-backend result comparison
- [ ] 4.5 Add skip logic for unsupported features (simd, threads, memory64, gc, eh, wasi)

## 5. Create Spec Test Cases

- [ ] 5.1 Create test for each spec file in `core/` directory
- [ ] 5.2 Run tests against interpreter, fix failures
- [ ] 5.3 Run tests against JIT, fix failures
- [ ] 5.4 Run tests against AOT, fix failures
- [ ] 5.5 Document any backend-specific differences

## 6. Remove Legacy Scripts

- [ ] 6.1 Remove `tests/wamr-test-suites/test_wamr.sh`
- [ ] 6.2 Remove `tests/wamr-test-suites/spec-test-script/runtest.py`
- [ ] 6.3 Remove `tests/wamr-test-suites/spec-test-script/all.sh`
- [ ] 6.4 Remove `tests/wamr-test-suites/spec-test-script/all.py`
- [ ] 6.5 Remove `tests/wamr-test-suites/wasi-test-script/`
- [ ] 6.6 Remove `tests/wamr-test-suites/wamr-compiler-test-script/`
- [ ] 6.7 Remove `tests/wamr-test-suites/requirement-engineering-test-script/`
- [ ] 6.8 Remove empty `spec-test-script/` directory

## 7. Final Integration

- [ ] 7.1 Run full `cargo test` to verify all spec tests pass
- [ ] 7.2 Update `tests/spec/src/lib.rs` to replace placeholder test
- [ ] 7.3 Verify CI integration (run tests in CI pipeline locally if possible)
