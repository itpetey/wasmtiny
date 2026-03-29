## Why

The WebAssembly memory64 proposal extends WebAssembly's linear memory from 4GiB to 64-bit addressing, enabling modules to use up to 4TiB of memory. This is essential for memory-intensive workloads like data processing, scientific computing, and large-scale caching. Currently, wasmtiny only supports 32-bit memory (up to 4GiB), limiting its usefulness for these scenarios.

## What Changes

- Add memory64 type to the WebAssembly module format (new memory type with i64 indices)
- Add 64-bit memory instructions: `i64.load`, `i64.store`, `i64.load8_s`, etc. (distinct from i32 variants)
- Add `memory.size` and `memory.grow` that return/work with i64
- Support memory64 with shared attribute (combines with threads proposal)
- Update the interpreter to handle 64-bit memory addressing
- Update the JIT compiler for 64-bit memory operations
- Update the AOT compiler for 64-bit memory operations

## Capabilities

### New Capabilities
- `wasm-memory64`: WebAssembly memory64 proposal implementation - 64-bit linear memory addressing
- `wasm-memory64-interpreter`: Memory64 support in the interpreter
- `wasm-memory64-jit`: Memory64 support in the JIT compiler
- `wasm-memory64-aot`: Memory64 support in the AOT compiler

### Modified Capabilities
(none - this adds new capabilities)

## Impact

- Modified: memory module, loader (parser/validator), interpreter, JIT, AOT
- New types: `Memory64` alongside existing `Memory32`
- Breaking: Memory index types change from u32 to u64 in relevant contexts
