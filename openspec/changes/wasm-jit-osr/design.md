## Context

The Rust WAMR rewrite includes a JIT compiler using Cranelift. Currently, functions are compiled either as baseline (fast compile, slower execution) or optimized (slower compile, faster execution) before being executed. OSR enables dynamic recompilation of hot functions mid-execution.

## Goals / Non-Goals

**Goals:**
- Implement OSR infrastructure for the JIT compiler
- Add hot function detection during execution
- Create transition mechanism from baseline to optimized code
- Maintain backward compatibility with existing JIT

**Non-Goals:**
- Support for de-optimization (optimized → baseline)
- OSR from interpreter to JIT (only JIT-to-JIT transitions)
- Cross-thread OSR

## Decisions

### 1. OSR Trigger Mechanism
**Decision**: Counter-based hot function detection in the interpreter.

Rationale: Simpler than profiling-based approach, sufficient for typical workloads.

### 2. OSR Entry Points
**Decision**: OSR at function boundaries (call/return), not arbitrary bytecode positions.

Rationale: Reduces complexity significantly. Most hot functions benefit from optimization on next invocation anyway.

### 3. Stack Frame Layout
**Decision**: Maintain separate stacks for baseline and optimized code with a shared metadata structure.

Rationale: Allows Cranelift to emit native code while maintaining compatibility with our existing stack unwinding.

## Risks / Trade-offs

- [Risk] OSR transition overhead could exceed benefits → [Mitigation] Only trigger OSR after function is called multiple times
- [Risk] Stack frame compatibility → [Mitigation] Use structured frame metadata, not raw pointer manipulation
- [Complexity] Implementing OSR correctly is complex → [Mitigation] Start with call-site OSR, expand later if needed