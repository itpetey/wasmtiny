## 1. Setup and Infrastructure

- [ ] 1.1 Create `/examples/` directory at project root
- [ ] 1.2 Create workspace `Cargo.toml` in `/examples/`
- [ ] 1.3 Add core WAMR C library as a dependency or build dependency
- [ ] 1.4 Configure build.rs for C compilation (or use wamr-sys crate)

## 2. Create First Example (Proof of Concept)

- [ ] 2.1 Create `examples/basic/` crate with Cargo.toml and src/main.rs
- [ ] 2.2 Implement basic Wasm loading using WAMR FFI bindings
- [ ] 2.3 Test that the example builds and runs
- [ ] 2.4 Add README.md to the basic example

## 3. Migrate Samples Directory

- [ ] 3.1 Create `examples/bh-atomic/` crate
- [ ] 3.2 Create `examples/file/` crate
- [ ] 3.3 Create `examples/import-func-callback/` crate
- [ ] 3.4 Create `examples/mem-allocator/` crate
- [ ] 3.5 Create `examples/multi-module/` crate
- [ ] 3.6 Create `examples/multi-thread/` crate
- [ ] 3.7 Create `examples/native-lib/` crate
- [ ] 3.8 Create `examples/shared-heap/` crate
- [ ] 3.9 Create `examples/shared-module/` crate
- [ ] 3.10 Create `examples/wasi-threads/` crate
- [ ] 3.11 Create remaining example crates for all other samples/ directories

## 4. Migrate Product-Mini Examples

- [ ] 4.1 Create `examples/product-mini/` crate for app-samples
- [ ] 4.2 Add platform-specific examples as conditionally compiled features
- [ ] 4.3 Add README.md explaining product-mini structure

## 5. Documentation and Migration Guide

- [ ] 5.1 Create `/examples/README.md` with overview and build instructions
- [ ] 5.2 Create `/examples/MIGRATION.md` with C to Rust migration guide
- [ ] 5.3 Add mapping table showing old -> new example correspondence
- [ ] 5.4 Update `/samples/README.md` with deprecation notice
- [ ] 5.5 Update `/product-mini/app-samples/README.md` with deprecation notice

## 6. Testing and Validation

- [ ] 6.1 Verify all example crates compile successfully
- [ ] 6.2 Run basic example and verify Wasm module execution
- [ ] 6.3 Verify cargo metadata shows all workspace members
- [ ] 6.4 Verify cargo doc generates documentation without errors