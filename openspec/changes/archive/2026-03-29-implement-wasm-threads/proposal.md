## Why

The WebAssembly threads proposal enables multi-threaded WebAssembly programs with shared memory and atomic operations. Currently, wasmtiny has partial shared memory support but lacks the atomic memory instructions (`memory.atomic.*`), atomic wait/notify operations (`atomic.wait`, `atomic.notify`), and proper thread synchronization primitives needed to run multi-threaded WASM.

## What Changes

- Add atomic memory instructions: `memory.atomic.notify`, `memory.atomic.wait32`, `memory.atomic.wait64`
- Add atomic load/store instructions: `i32.atomic.load`, `i64.atomic.load`, `i32.atomic.store`, etc.
- Add atomic read-modify-write instructions: `atomic.rmw.add`, `atomic.rmw.sub`, `atomic.rmw.and`, `atomic.rmw.or`, `atomic.rmw.xor`, `atomic.rmw.xchg`, `atomic.rmw.cmpxchg`
- Implement thread-local store (TLS) for thread-specific data
- Add proper memory model synchronization for shared memory
- Update the interpreter to handle atomic operations
- Update the JIT compiler to emit atomic operations
- Update the AOT compiler to handle atomic operations

## Capabilities

### New Capabilities
- `wasm-threads`: WebAssembly threads proposal implementation - shared memory with atomic operations and thread synchronization
- `wasm-atomics`: Atomic memory operations (load, store, read-modify-write) for the interpreter
- `wasm-atomics-jit`: Atomic memory operations for the JIT compiler
- `wasm-atomics-aot`: Atomic memory operations for the AOT compiler

### Modified Capabilities
(none - this is adding new capabilities)

## Impact

- New modules: `src/runtime/atomics.rs`, `src/interpreter/atomics.rs`
- Modified: interpreter instruction decoder, JIT emitter, AOT loader
- Dependencies: None (implementing from scratch using std::sync::atomic)
