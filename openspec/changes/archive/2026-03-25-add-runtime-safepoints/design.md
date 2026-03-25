## Context

`wasmtiny` can instantiate modules and execute exports, but it does not yet model a paused guest. Selium's newer runtime designs rely on cooperative yielding and resumable host interactions, while future runtime draining and migration also need a safe point where guest-visible state is stable.

## Goals / Non-Goals

**Goals:**
- Define a runtime-native suspend/resume model that works for both interpreter and JIT execution.
- Ensure suspension happens only at explicit, well-defined safepoints.
- Keep resumable state in canonical runtime structures rather than embedding it in transient native frames.
- Make unsupported states fail clearly.

**Non-Goals:**
- Full pre-emptive interruption at arbitrary machine instructions.
- Solving snapshot serialisation in this change.
- Defining Selium capability or queue semantics.

## Decisions

### 1. Cooperative safepoints only
Suspension will only occur at explicit runtime safepoints such as hostcall boundaries, yield points, or other designated checks. This keeps the semantic state tractable and avoids arbitrary machine-frame capture.

### 2. Canonical resumable state is runtime-owned
The source of truth for a suspended guest will be runtime data structures representing pc, locals, operand stack, memory/table/global references, and pending hostcall state. JIT state must be reconstructable from that representation rather than being the representation.

### 3. Suspension handles are opaque and single-owner
The public API should hand higher layers an opaque suspended-instance token/handle rather than exposing internal frame structures. This makes it easier to evolve the implementation and reduces accidental misuse.

### 4. Interpreter and JIT share the same lifecycle semantics for supported safepoints
The interpreter may be the first implementation path, but JIT execution must honour the same handle ownership, epoch validation, and explicit failure semantics for supported safepoints so higher layers do not need silent engine-specific fallbacks. Unsupported JIT suspension paths, such as pending mid-function hostcalls, must fail explicitly.

## Risks / Trade-offs

- [Safepoint overhead] -> Keep safepoints explicit and sparse, then measure interpreter/JIT impact.
- [JIT mismatch with canonical state] -> Make the resumable state machine runtime-owned and regenerate JIT artefacts as needed.
- [API lock-in] -> Use opaque handles and narrowly defined lifecycle calls.
- [Partially-supported hostcalls] -> Reject unsupported suspension paths explicitly.
