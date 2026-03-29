## Context

The WebAssembly memory64 proposal (W3C recommendation) extends WebAssembly's linear memory addressing from 32-bit to 64-bit:

- **Memory64 type**: New memory type `memory64` with i64 indices
- **Larger limits**: Up to 4TiB (2^52 bytes) of addressable memory
- **New limits page size**: 64KiB pages (vs 32-bit memory's 64KiB)
- **Shared variant**: Can combine with threads proposal for shared memory64

The proposal introduces:
- `memory.init`, `data.drop` - for 64-bit memory
- `memory.size`, `memory.grow` - return i64
- Load/store instructions with i64 address arguments

Current wasmtiny: 32-bit memory only, max 4GiB, uses u32 for all memory operations.

## Goals / Non-Goals

**Goals:**
- Support memory64 type in module format
- Support memory64 with up to 4TiB addressable space
- Implement 64-bit variants of memory instructions
- Support memory64 with shared attribute (memory64 + threads)
- Ensure all three backends (interpreter, JIT, AOT) support memory64

**Non-Goals:**
- Implement the 4GiB+ garbage collection proposal (separate)
- Support memory64 in WASI (out of scope for wasmtiny)
- Thread spawning - just the memory capability

## Decisions

### 1. Memory Type Architecture

**Decision:** Implement memory64 as separate type from memory32

**Rationale:**
- Keeps existing memory32 code path unchanged
- Cleaner separation of concerns
- Easier to maintain and debug

**Memory representation:**
```rust
enum Memory {
    Memory32(Memory32),
    Memory64(Memory64),
}

struct Memory64 {
    pages: u64,           // 64-bit page count
    max_pages: Option<u64>,
    shared: bool,         // from threads proposal
    data: Vec<u8>,        // underlying storage
}
```

### 2. Instruction Handling

**Decision:** Add new instruction variants for 64-bit operations

**Rationale:**
- i64.load is distinct from i32.load in the spec (different opcode)
- Cleaner than branching on memory type in existing instructions
- Easier to validate requirements

**New instructions needed:**
```
i64.load    i64.load8_s  i64.load8_u  i64.load16_s  i64.load16_u  i64.load32_s  i64.load32_u
i64.store   i64.store8   i64.store16  i64.store32
memory.size    // returns i64 for memory64
memory.grow    // takes i64 for memory64
memory.init    // for data segments
data.drop      // for data segments
```

### 3. Address Space Allocation

**Decision:** Use sparse memory with on-demand allocation

**Rationale:**
- Allocating 4TiB upfront is impossible
- Use guard pages and demand paging
- Platform: Use memory-mapped files or sparse files

**Implementation:**
- Use Rust's `Vec<u8>` with capacity planning
- Pre-allocate guard regions (1GB minimum)
- Use `memory::map()` on Unix for efficient allocation

### 4. Interoperability with memory32

**Decision:** Module can have either memory32 OR memory64, not both

**Rationale:**
- Matches WebAssembly spec
- Simpler validation
- Reduces complexity

### 5. JIT/AOT Considerations

**Decision:** Use 64-bit addressing in LLVM IR

**Rationale:**
- LLVM natively supports 64-bit
- x86-64, aarch64 have 64-bit address spaces
- Simpler code generation

## Risks / Trade-offs

- **[Risk]** Address space exhaustion on 32-bit hosts
  - **Mitigation:** Detect at module load time, fail gracefully

- **[Risk]** Performance of sparse memory allocation
  - **Mitigation:** Use platform-native large page support
  - Profile and optimize hotspots

- **[Risk]** Compatibility with existing memory code
  - **Mitigation:** Strict separation, shared interfaces only
  - Extensive testing required

- **[Risk]** Atomic operations on memory64
  - **Mitigation:** Memory64 + threads requires sequential consistency
  - Use proper atomic primitives

## Migration Plan

1. Add memory64 type to loader (parser, validator)
2. Create memory64 runtime module
3. Add 64-bit memory instructions to interpreter
4. Update JIT for 64-bit addressing
5. Update AOT for 64-bit addressing
6. Run memory64 spec tests
7. Remove memory64 from skipped spec tests

## Open Questions

- Should we support converting memory32 to memory64 at runtime?
- What should be the default guard page size for memory64?
- How to handle out-of-memory gracefully?
