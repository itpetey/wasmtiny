## Why

The wasm-micro-runtime (WAMR) is implemented entirely in C, which limits its ability to leverage modern Rust features such as memory safety guarantees, zero-cost abstractions, and robust concurrency primitives. Rewriting core components in Rust will improve code safety, maintainability, and performance while opening opportunities for safer FFI interfaces and better integration with the Rust ecosystem.

## What Changes

- Rewrite core WAMR runtime components in idiomatic Rust with focus on:
  - Memory-safe execution engine
  - Type-safe WASM module loading and validation
  - Modern iterator and error handling patterns
- Replace C data structures with Rust equivalents (Vec, Box, Arc, etc.)
- Implement comprehensive unit tests using Rust's built-in testing framework
- Expose Rust-native APIs while maintaining C compatibility via unsafe FFI
- Apply Rust idioms: Result/Option instead of error codes, traits for polymorphism, lifetimes for ownership
- Ignore WASI extensions entirely - no WASI support in the rewrite

## Capabilities

### New Capabilities
- `wasm-runtime-core`: Core WAMR runtime engine rewritten in Rust with memory-safe execution
- `wasm-module-loader`: WASM module parsing, validation, and loading with comprehensive error handling
- `wasm-interpreter`: Thread-safe interpreter with both classic and fast execution modes
- `wasm-aot-runtime`: Ahead-of-time compiled WASM runtime support
- `wasm-fast-jit`: Fast JIT compilation infrastructure

### Modified Capabilities
<!-- No existing capabilities - this is a new rewrite -->

## Impact

- Primary: `core/iwasm/` - complete rewrite of all C modules
- Build system: Replace cmake/SConscript builds with Cargo workspace
- Testing: Replace C unit tests with Rust `#[cfg(test)]` modules
- C API: Maintain `wasm_c_api.h` compatibility through unsafe FFI bindings
