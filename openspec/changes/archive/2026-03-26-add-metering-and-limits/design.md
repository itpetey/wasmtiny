## Context

`wasmtiny` currently has no unified runtime accounting model and no first-class instance budget enforcement. Selium can choose policies, but the runtime itself must expose trustworthy counters and enforce configured ceilings if the host is going to place and isolate work safely.

## Goals / Non-Goals

**Goals:**
- Define a runtime-owned stats surface for execution and memory usage.
- Define configurable per-instance limits enforced by the runtime.
- Keep metering separate from Selium-specific policy decisions.
- Ensure interpreter and JIT paths enforce the same semantics.

**Non-Goals:**
- Full cluster scheduling logic.
- Billing policy or multi-tenant accounting formats.
- Perfect cycle-accurate CPU accounting in the first version.

## Decisions

### 1. Metering and limits are separate but adjacent runtime concerns
Metering provides observation; limits provide enforcement. They should share underlying counters where appropriate, but the API should keep those roles distinct.

### 2. Limits are configured per instance
The runtime should accept per-instance budget configuration when creating or starting an instance rather than baking in one global process-wide ceiling.

### 3. Enforcement semantics must match across execution engines
Interpreter and JIT execution must report counters and enforce budgets using the same observable rules so higher layers do not need engine-specific handling.

### 4. Limit breaches are explicit runtime failures
The runtime should expose a deterministic error or trap when a budget is exceeded instead of letting execution continue in a degraded or partially-accounted state.

## Risks / Trade-offs

- [Accounting overhead] -> Start with coarse but trustworthy counters, then optimise hot paths.
- [JIT/interpreter drift] -> Drive both engines from shared budget semantics and shared tests.
- [API overreach] -> Expose stats and budget primitives, not Selium scheduling policy.
- [False precision expectations] -> Document exactly what each counter means.
