## Context

The WebAssembly threads proposal (formerly known as threads/channels) adds:
1. **Shared Linear Memory** - memory can be marked as shared (via `shared` attribute in memory type)
2. **Atomic Operations** - atomic load, store, and read-modify-write operations on shared memory
3. **Wait and Notify** - `atomic.wait` and `atomic.notify` instructions for synchronization

wasmtiny currently has:
- Partial shared memory support (`SharedMemoryRegistry`, atomic primitives in `suspend.rs`)
- Basic reference types (FuncRef, ExternRef)
- No atomic memory instructions implemented

The threads proposal is at W3C recommendation status (finalized).

## Goals / Non-Goals

**Goals:**
- Implement all atomic memory instructions (32-bit and 64-bit variants)
- Implement atomic.wait and atomic.notify
- Support shared linear memory
- Ensure interpreter, JIT, and AOT all support threads
- Pass WebAssembly threads spec tests

**Non-Goals:**
- Implement WebAssembly threads (worker threads) - just the memory/atomic instructions
- Implement high-level threading APIs (this is WASI territory)
- Thread spawning from Rust host (keep it simple for now)

## Decisions

### 1. Atomic Implementation Approach

**Decision:** Use Rust's `std::sync::atomic` primitives

**Rationale:**
- No external dependencies
- Well-tested, cross-platform
- Matches the memory model requirements of the spec
- Already partially used in `suspend.rs`

**Atomic operations needed:**
```
i32.atomic.load    i64.atomic.load    i32.atomic.store    i64.atomic.store
i32.atomic.load8_u i64.atomic.load8_u i32.atomic.store8   i64.atomic.store8
i32.atomic.load16_u i64.atomic.load16_u i32.atomic.store16 i64.atomic.store16
i32.atomic.load32_u                           i32.atomic.store32

i32.atomic.rmw.add    i64.atomic.rmw.add
i32.atomic.rmw.sub    i64.atomic.rmw.sub
i32.atomic.rmw.and    i64.atomic.rmw.and
i32.atomic.rmw.or     i64.atomic.rmw.or
i32.atomic.rmw.xor    i64.atomic.rmw.xor
i32.atomic.rmw.xchg   i64.atomic.rmw.xchg
i32.atomic.rmw.cmpxchg i64.atomic.rmw.cmpxchg

memory.atomic.notify
memory.atomic.wait32
memory.atomic.wait64
```

### 2. Instruction Encoding

**Decision:** Use opcode 0xFE (atomic memory operations) as specified in the proposal

**Rationale:**
- Standard WebAssembly encoding
- Aligns with official spec
- Allows parser reuse

### 3. Interpreter Implementation

**Decision:** Add atomic operations to existing interpreter with separate atomic instruction handler

**Rationale:**
- Keeps code organized
- Allows atomic-specific error handling
- Matches existing instruction handling pattern

### 4. JIT Implementation

**Decision:** Use LLVM atomic intrinsics where possible

**Rationale:**
- LLVM has built-in atomic operations
- Ensures correct memory ordering
- Better optimization opportunities

### 5. AOT Implementation

**Decision:** Generate atomic operations using compiler intrinsic calls

**Rationale:**
- Simpler than LLVM IR generation for atomics
- Can use same runtime functions as interpreter

## Risks / Trade-offs

- **[Risk]** Memory ordering - WebAssembly uses sequential consistency by default
  - **Mitigation:** Use `Ordering::SeqCst` for all atomic operations initially, optimize later

- **[Risk]** Waiting on memory - atomic.wait requires blocking the thread
  - **Mitigation:** Use `std::thread::park` with a parking lot primitive

- **[Risk]** Shared memory validation - must ensure memory is actually shared
  - **Mitigation:** Add validation in module loading for atomic operations

- **[Risk]** Performance - atomic operations are slower than non-atomic
  - **Mitigation:** Document this, allow non-atomic fallback for single-threaded

## Migration Plan

1. Add atomic instruction enum variants to `interpreter/instructions.rs`
2. Add atomic instruction parsing to `loader/parser.rs`
3. Create `runtime/atomics.rs` with atomic operation implementations
4. Update interpreter execution to handle atomic instructions
5. Update JIT emitter for atomic operations
6. Update AOT loader for atomic operations
7. Run threads spec tests
8. Remove "threads" from skipped spec tests

## Open Questions

- Should we add ` #[deny(unsafe_op_in_unsafe_fn)]` for atomic code?
- Do we need to expose thread creation to the host, or just enable WASM threads?
- How to handle memory.grow on shared memory (has different semantics)?
