## Why

The WebAssembly SIMD (Single Instruction Multiple Data) proposal enables vector operations that process multiple data elements in parallel with a single instruction. This is essential for performance-critical applications like video processing, machine learning, cryptography, and scientific computing. SIMD can provide 4-16x speedups for appropriate workloads. Currently, wasmtiny does not support SIMD, limiting its ability to run performance-sensitive workloads.

## What Changes

- Add v128 type (128-bit vector type) to the type system
- Add SIMD instruction opcode (0xFD) and sub-operations
- Implement integer SIMD operations: add, sub, mul, div, min, max, abs, shifts, etc.
- Implement floating-point SIMD operations: add, sub, mul, div, min, max, sqrt, abs, round, etc.
- Implement SIMD logical operations: and, or, xor, not
- Implement SIMD shuffle and swizzle operations
- Implement SIMD load/store operations
- Implement SIMD reduce operations (any_true, all_true)
- Update interpreter to handle SIMD instructions
- Update JIT compiler for SIMD operations
- Update AOT compiler for SIMD operations

## Capabilities

### New Capabilities
- `wasm-simd`: WebAssembly SIMD proposal implementation - 128-bit vector operations
- `wasm-simd-interpreter`: SIMD support in the interpreter
- `wasm-simd-jit`: SIMD support in the JIT compiler
- `wasm-simd-aot`: SIMD support in the AOT compiler

### Modified Capabilities
(none - this adds new capabilities)

## Impact

- New type: `v128` (128-bit vector) alongside existing numeric types
- New instruction category: ~150 SIMD instructions
- Modified: loader, interpreter, JIT, AOT
- Dependencies: None (implement using native Rust/SIMD intrinsics or software emulation)
