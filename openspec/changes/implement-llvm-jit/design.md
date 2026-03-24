## Context

The wasmtiny runtime needs an LLVM-based JIT compiler to achieve high-performance execution. WAMR's llvm-jit uses LLVM's ORC (On-Request Compilation) API to compile WASM modules to native code at runtime. This provides advanced optimizations (loop vectorization, constant propagation, dead code elimination, inlining) that are not available in fast-jit.

Reference implementation: wasm-micro-runtime's `core/iwasm/compilation/` and `core/iwasm/jit/`.

## Goals / Non-Goals

**Goals:**
- Integrate LLVM as a JIT compilation backend via llvm-sys crate
- Translate WASM bytecode to LLVM IR using wasm-micro-runtime as reference
- Use LLVM's ORC JIT API for runtime compilation and execution
- Achieve feature parity with WAMR's llvm-jit for running regression tests

**Non-Goals:**
- AOT (ahead-of-time) compilation - separate from JIT
- SIMD/vector optimizations - future work
- Multi-threading optimizations - future work

## Decisions

### 1. Use LLVM ORC2 API vs legacy ORC1
**Alternative:** Use ORC1 (older, simpler API)

Decision: Use ORC2 (LLVM 14+). ORC2 provides better performance and more features. WAMR has migrated to ORC2.

### 2. WASM→IR translation strategy
**Alternative:** Reuse fast-jit IR then convert to LLVM IR

Decision: Direct WASM→LLVM IR translation. This avoids intermediate representation overhead and matches WAMR's approach in `wasm_to_llvm_ir()`.

### 3. LLVM linking strategy
**Alternatives:**
- Lazy compilation (compile each function on first call)
- Eager compilation (compile all functions upfront)

Decision: Lazy compilation via ORC's JITDylib. Functions are compiled on-demand, matching WAMR's behavior and reducing startup time.

### 4. Memory access handling
**Alternatives:**
- Emulate WASM memory with custom allocator
- Use LLVM's built-in memory operations

Decision: Custom memory accessor functions. WASM linear memory is accessed via runtime-provided functions (similar to WAMR's approach), allowing bounds checking to be handled at the runtime level.

### 5. Function call handling
**Alternatives:**
- Indirect calls through runtime dispatcher
- Direct compilation of function addresses

Decision: Use LLVM's helper functions for cross-module calls. For imported functions, use runtime-provided trampolines (matching WAMR's approach).

## Risks / Trade-offs

- [Risk] LLVM version compatibility → Mitigation: Use feature-gated llvm-sys with version detection; test against multiple LLVM versions
- [Risk] Long compilation times → Mitigation: Use lazy compilation; fast-jit can be used as Tier-1 while llvm-jit compiles in background
- [Risk] Large binary size due to LLVM → Mitigation: Statically link LLVM to avoid runtime dependency; LLVM can be omitted via feature flag for minimal builds

## Open Questions

- Should we support both LLVM JIT and fast-jit simultaneously (tiered)?
  - Yes: fast-jit for quick warmup, llvm-jit for hot functions
- How to handle WASM→WASM calls in LLVM IR?
  - Use function addresses resolved at link time via ORC's symbol resolution

## Migration Plan

1. Add llvm-sys dependency to Cargo.toml (feature-gated)
2. Create src/jit/llvm_backend.rs with ORC integration
3. Create src/jit/wasm_to_llvm.rs for IR translation
4. Integrate with WasmApplication
5. Run llvm-jit regression tests to verify