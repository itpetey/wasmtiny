## Why

wasmtiny currently lacks LLVM-based JIT compilation. While fast-jit provides baseline performance, many production workloads require the advanced optimizations that LLVM provides (loop vectorization, constant propagation, inlining, etc.). Adding llvm-jit enables 25+ regression tests that require llvm-jit mode and provides a high-performance execution tier.

## What Changes

- Integrate LLVM (via `llvm-sys` crate) as a compilation backend
- Create WASM→LLVM IR translation layer
- Implement JIT compilation pipeline using LLVM's ORC (On-Request Compilation) runtime
- Add llvm-jit as an execution mode alongside fast-jit and interpreter
- Enable execution of regression tests requiring llvm-jit runtime

## Capabilities

### New Capabilities
- `llvm-jit-compiler`: LLVM-based JIT compiler using LLVM's ORC JIT API
- `wasm-llvm-ir-translator`: Translate WASM bytecode to LLVM IR
- `llvm-jit-runtime`: Runtime support for executing LLVM-compiled code

### Modified Capabilities
- (none - this is a new capability, not modifying existing spec behavior)

## Impact

- New dependency: `llvm-sys` crate for LLVM bindings
- New module: `src/jit/llvm_backend.rs` - LLVM integration
- New module: `src/jit/wasm_to_llvm.rs` - WASM to LLVM IR translation
- Modified: `tests/regression.rs` - unpark llvm-jit regression tests