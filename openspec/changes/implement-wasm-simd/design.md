## Context

The WebAssembly SIMD proposal (W3C recommendation) adds:
- **v128 type**: 128-bit vector type holding 16x i8, 8x i16, 4x i32, 2x i64, or 4x f32, 2x f64
- **SIMD operations**: ~150 instructions covering integer, floating-point, and logical operations
- **Lane operations**: operate on multiple data elements in parallel

SIMD is particularly important for:
- Image/video processing (blurs, filters, color conversion)
- Cryptography (AES-NI-like operations, SHA)
- Machine learning (matrix multiplications, activations)
- String processing (search, case conversion)

The spec is structured as several phases (128-bit, relaxed SIMD) - we implement the core 128-bit proposal.

## Goals / Non-Goals

**Goals:**
- Implement v128 type in type system
- Implement all 128-bit SIMD instructions
- Support both integer and floating-point SIMD operations
- Ensure interpreter, JIT, and AOT support SIMD
- Pass WebAssembly SIMD spec tests

**Non-Goals:**
- Relaxed SIMD (additional floating-point behaviors)
- 256-bit/512-bit vectors (not in WASM spec)
- SIMD-specific memory allocation strategies

## Decisions

### 1. v128 Representation

**Decision:** Use `[u8; 16]` as internal v128 representation

**Rationale:**
- Simple, portable, no alignment issues
- Easy to implement lane operations
- Matches WebAssembly's little-endian lane ordering
- Can optimize to SIMD registers where available

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct V128(pub [u8; 16]);

impl V128 {
    // Lane accessors for different element sizes
    pub fn i8_lane(self, idx: u8) -> i8 { ... }
    pub fn i16_lane(self, idx: u8) -> i16 { ... }
    pub fn i32_lane(self, idx: u8) -> i32 { ... }
    pub fn i64_lane(self, idx: u8) -> i64 { ... }
    pub fn f32_lane(self, idx: u8) -> f32 { ... }
    pub fn f64_lane(self, idx: u8) -> f64 { ... }
}
```

### 2. Implementation Strategy

**Decision:** Start with portable software implementation, add hardware SIMD as optimization

**Rationale:**
- Works on all platforms immediately
- Easier to debug and test
- Can add platform-specific intrinsics later (std::arch::x86, std::arch::aarch64)

**Portable implementation:**
```rust
fn i32x4_add(a: V128, b: V128) -> V128 {
    let mut result = [0u8; 16];
    for i in 0..4 {
        let ai = i32::from_le_bytes([a.0[i*4], a.0[i*4+1], a.0[i*4+2], a.0[i*4+3]]);
        let bi = i32::from_le_bytes([b.0[i*4], b.0[i*4+1], b.0[i*4+2], b.0[i*4+3]]);
        let ri = ai.wrapping_add(bi);
        result[i*4..i*4+4].copy_from_slice(&ri.to_le_bytes());
    }
    V128(result)
}
```

### 3. Instruction Categories

SIMD instructions are organized into categories:

| Category | Count | Examples |
|----------|-------|----------|
| Load/Store | 8 | v128.load, v128.load8x8_s, v128.store |
| Const | 1 | v128.const |
| Lane ops | 28 | i8x16.extract_lane, i32x4.replace_lane |
| Integer arithmetic | 32 | i8x16.add, i16x8.mul, i32x4.min_s |
| Float arithmetic | 24 | f32x4.add, f64x2.sqrt, f32x4.neg |
| Logical | 4 | v128.and, v128.or, v128.xor, v128.not |
| Shift | 16 | i8x16.shl, i16x8.shr_u |
| Shuffle | 2 | i8x16.shuffle, i8x16.swizzle |
| Reduce | 10 | i8x16.any_true, i32x4.all_true |

### 4. JIT Implementation

**Decision:** Use LLVM's SIMD intrinsics where available

**Rationale:**
- LLVM generates optimal SIMD code
- Auto-vectorization when explicit SIMD not available
- Cross-platform support

```rust
// Example JIT translation
i32x4_add(a, b) -> %result = add <4 x i32> %a, %b
```

### 5. NaN Handling

**Decision:** Follow WebAssembly spec for NaN propagation

**Rationale:**
- WASM defines specific NaN handling for SIMD
- Must match spec for conformance
- Tests will verify correct behavior

## Risks / Trade-offs

- **[Risk]** Performance of software SIMD
  - **Mitigation:** Start portable, add platform intrinsics later
  - Profile to find hotspots

- **[Risk]** Large instruction set (~150 instructions)
  - **Mitigation:** Implement in batches by category
  - Use macros to reduce boilerplate

- **[Risk]** Floating-point edge cases (NaN, -0, inf)
  - **Mitigation:** Follow spec exactly
  - Extensive test coverage

- **[Risk]** JIT code generation complexity
  - **Mitigation:** Use LLVM intrinsics, not manual codegen

## Migration Plan

1. Add v128 type to loader (parser, validator)
2. Create runtime/simd.rs with V128 implementation
3. Add SIMD instructions to interpreter
4. Update JIT for SIMD operations
5. Update AOT for SIMD operations
6. Run SIMD spec tests
7. Remove SIMD from skipped spec tests

## Open Questions

- Should we use platform-specific intrinsics from the start?
- How to handle relaxed SIMD (future phase)?
- What's the minimum viable implementation to pass spec tests?
