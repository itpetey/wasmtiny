## 1. Project Setup

- [x] 1.1 Create `core/rust-wamr/` directory structure
- [x] 1.2 Create `Cargo.toml` with workspace manifest
- [x] 1.3 Create base `lib.rs` with module declarations
- [x] 1.4 Add dependencies (memchr, byteorder, leb128)
- [x] 1.5 Configure `.cargo/config.toml` for cross-compilation targets

## 2. Core Types (wasm-runtime-core)

- [x] 2.1 Define `WasmError` enum with `Result<T>` type alias
- [x] 2.2 Define `TrapCode` enum for trap types
- [x] 2.3 Implement `ValType`, `RefType`, and `NumType` enums
- [x] 2.4 Implement `FunctionType` struct with parameter/result types
- [x] 2.5 Implement `GlobalType`, `TableType`, `MemoryType` structs
- [x] 2.6 Implement `Table`, `Memory`, `Global` structs with ownership
- [x] 2.7 Implement `ExportType` and `ImportType` structs
- [x] 2.8 Implement `Module` struct with all WASM sections
- [x] 2.9 Implement `Instance` struct with runtime state
- [x] 2.10 Implement thread-safety traits (`Send`, `Sync`) where applicable

## 3. Module Loader (wasm-module-loader)

- [x] 3.1 Implement binary reader with `Read` trait
- [x] 3.2 Parse magic number and version
- [x] 3.3 Parse Type section into `Vec<FunctionType>`
- [x] 3.4 Parse Import section into `Vec<Import>`
- [x] 3.5 Parse Function section with type indices
- [x] 3.6 Parse Table section
- [x] 3.7 Parse Memory section
- [x] 3.8 Parse Global section
- [x] 3.9 Parse Export section
- [x] 3.10 Parse Code section with function bodies
- [x] 3.11 Parse Data section
- [x] 3.12 Parse Name section (optional)
- [x] 3.13 Implement validation for type checking
- [x] 3.14 Implement validation for stack polymorphism
- [x] 3.15 Implement validation for br_table targets
- [ ] 3.16 Implement streaming parser (optional)

## 4. Stack-Based Interpreter (wasm-interpreter)

- [x] 4.1 Implement operand stack (`Vec<WasmValue>`)
- [x] 4.2 Implement control stack (`Vec<ControlFrame>`)
- [x] 4.3 Implement `Frame` struct with labels
- [x] 4.4 Implement numeric instructions (i32, i64, f32, f64)
- [x] 4.5 Implement memory instructions (load/store)
- [x] 4.6 Implement control flow instructions (block, loop, if, br, br_if, br_table)
- [x] 4.7 Implement call instructions (call, call_indirect)
- [x] 4.8 Implement local instructions (local.get, local.set, local.tee)
- [x] 4.9 Implement global instructions
- [x] 4.10 Implement table instructions
- [x] 4.11 Implement memory.grow and memory.size
- [x] 4.12 Implement parametric instructions (drop, select)
- [x] 4.13 Implement reference instructions
- [ ] 4.14 Implement host function call dispatch
- [x] 4.15 Implement trap handling and propagation
- [x] 4.16 Implement stack overflow detection

## 5. Fast Interpreter Mode

- [ ] 5.1 Implement register-based IR representation
- [ ] 5.2 Convert bytecode to register IR
- [ ] 5.3 Optimize redundant loads/stores
- [ ] 5.4 Optimize constant folding
- [ ] 5.5 Integrate fast interpreter with core interpreter

## 6. AOT Runtime (wasm-aot-runtime)

- [x] 6.1 Define AOT module format structures
- [x] 6.2 Implement AOT loader
- [ ] 6.3 Implement native function table dispatch
- [ ] 6.4 Implement memory management for AOT
- [ ] 6.5 Implement table management
- [ ] 6.6 Implement global variable access
- [ ] 6.7 Implement trap handling

## 7. Fast JIT (wasm-fast-jit) - Using Cranelift

- [ ] 7.1 Add cranelift and cranelift-wasm dependencies to Cargo.toml
- [ ] 7.2 Implement WASM-to-Cranelift IR translation
- [ ] 7.3 Configure cranelift for WASM calling convention
- [ ] 7.4 Implement code cache for compiled functions
- [ ] 7.5 Implement trampoline generation for indirect calls
- [ ] 7.6 Implement OSR support (optional)
- [ ] 7.7 Add unit tests for JIT compilation

## 8. C API Compatibility

- [x] 8.1 Create `c-api/` module with FFI declarations
- [x] 8.2 Implement `wasm_module_new` binding
- [x] 8.3 Implement `wasm_instance_new` binding
- [x] 8.4 Implement `wasm_func_call` binding
- [x] 8.5 Implement memory FFI bindings
- [x] 8.6 Implement table FFI bindings
- [x] 8.7 Implement global FFI bindings
- [x] 8.8 Implement error propagation from Rust to C

## 9. Testing Infrastructure

- [x] 9.1 Add `#[cfg(test)]` modules to core types
- [x] 9.2 Add unit tests for binary parsing
- [x] 9.3 Add unit tests for validation
- [x] 9.4 Add unit tests for interpreter execution
- [x] 9.5 Add unit tests for memory operations
- [x] 9.6 Add unit tests for error handling
- [ ] 9.7 Create integration test harness

## 10. Build System

- [x] 10.1 Configure cargo build profiles
- [x] 10.2 Add cross-compilation support
- [x] 10.3 Document build requirements
- [ ] 10.4 Create CMakeLists.txt for C integration (optional)
