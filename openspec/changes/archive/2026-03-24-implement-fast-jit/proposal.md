## Why

The wasmtiny runtime lacks a working JIT compiler. The existing `src/jit/` module only implements partial IR translation for a handful of opcodes and has no native code generation backend. This prevents wasmtiny from running 40+ regression tests that require fast-jit mode, limiting its utility for performance-sensitive workloads.

## What Changes

- Implement a complete fast-jit compiler that translates WASM to native x64 machine code
- Replace the existing stub JIT module with a full code generation pipeline
- Add tiered compilation support (baseline → optimized) for progressive optimization
- Enable execution of regression tests that currently require fast-jit runtime

## Capabilities

### New Capabilities
- `fast-jit-compiler`: Full WASM-to-x64 JIT compiler with register allocation and instruction emission
- `jit-runtime`: Runtime support for compiled code execution, including function call stubs and memory accessors
- `osr-support`: On-Site Replacement infrastructure for tier upgrading during execution

### Modified Capabilities
- (none - this is a new capability, not modifying existing spec behavior)

## Impact

- New module: `src/jit/compiler.rs` - expands from IR translation to full x64 codegen
- New module: `src/jit/emitter.rs` - native instruction emission
- New module: `src/jit/regalloc.rs` - register allocation for x64
- Modified: `src/jit/runtime.rs` - add execution engine for JIT-compiled code
- Modified: `tests/regression.rs` - unpark fast-jit regression tests