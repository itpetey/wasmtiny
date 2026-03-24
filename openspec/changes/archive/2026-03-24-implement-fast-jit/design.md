## Context

The wasmtiny runtime currently lacks a working JIT compiler. The existing `src/jit/` module in `compiler.rs` only translates a handful of WASM opcodes to a custom IR format, with no native code generation backend. This blocks 40+ regression tests requiring fast-jit mode.

Reference implementation: [wasm-micro-runtime (WAMR)](https://github.com/bytecodealliance/wasm-micro-runtime) - specifically the `core/fast-jit/` subsystem.

## Goals / Non-Goals

**Goals:**
- Implement full fast-jit compiler that compiles WASM to native x64 machine code
- Achieve feature parity with WAMR's fast-jit for running regression tests
- Maintain interpreter as fallback for unsupported opcodes
- Support tiered compilation (baseline → optimized) for future optimization

**Non-Goals:**
- LLVM-based JIT (llvm-jit) - separate effort
- SIMD/vector instructions - V128 - future work
- GC-enabled runtime support - future work
- AOT compilation - separate from JIT

## Decisions

### 1. Use direct WASM-to-x64 compilation
**Alternative:** Reuse existing IR layer then emit x64

Decision: Skip internal IR, compile WASM bytecode directly to x64. WAMR's fast-jit takes this approach for lower overhead. The existing `translate_wasm_to_ir()` stub will be replaced with direct codegen.

### 2. Register allocation strategy
**Alternatives:**
- Linear scan register allocation
- Graph-coloring (Chaitin-Briggs)
- Stack-based (Stackify-style)

Decision: Use linear scan register allocation. It's simpler to implement, provides good results for WASM's short-lived values, and is what WAMR uses for fast-jit baseline tier.

### 3. Memory access via direct x64 addressing
**Alternatives:**
- Indirect memory access through runtime helpers
- Bounds-checked direct addressing

Decision: Generate inline bounds checks with direct memory addressing. WAMR shows this pattern achieves ~10% faster memory operations than helper calls.

### 4. Function call handling
**Alternatives:**
- All calls go through interpreter
- JIT-compiled calls for direct functions, helper for imports

Decision: For direct WASM→WASM calls, emit direct jumps. For host imports, use runtime trampolines. Matches WAMR's fast-jit calling convention.

## Risks / Trade-offs

- [Risk] x64 instruction encoding is complex → Mitigation: Use existing crate like `codegen` or implement subset of x64 first
- [Risk] Missing WASM opcodes will cause runtime failures → Mitigation: Fall back to interpreter for unimplemented opcodes (same as WAMR)
- [Risk] Security: JIT code must be marked executable only when needed → Mitigation: Use standard W^X patterns, mark pages RWX only during compilation

## Open Questions

- Should we use an external crate for x64 codegen (e.g., `cranelift-codegen`) or implement our own?
  - Recommendation: Implement our own for control/maintainability, using WAMR's implementation as reference
- How to handle stack unwinding for exceptions?
  - Follow WAMR: emit stack metadata alongside code for runtime introspection