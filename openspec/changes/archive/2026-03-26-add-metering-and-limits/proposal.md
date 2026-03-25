## Why

Selium can only enforce placement and isolation policy if the runtime exposes reliable execution accounting and enforces per-instance budgets. `wasmtiny` needs first-class metering and limits so higher layers stop guessing about runtime cost and safety boundaries.

## What Changes

- Add runtime metering hooks for execution, memory usage, and other instance-level counters.
- Add configurable per-instance limits for memory and execution budgets.
- Require limit breaches to fail explicitly and predictably.
- Expose runtime stats APIs without baking Selium-specific policy into the engine.

## Capabilities

### New Capabilities
- `runtime-metering`: Runtime counters and stats for instance execution and resource usage.
- `instance-limits`: Runtime-enforced per-instance limits for memory and execution budgets.

### Modified Capabilities

## Impact

- Affects interpreter and JIT execution paths, instance configuration, and public runtime APIs.
- Provides the accounting substrate higher-level schedulers and supervisors need.
- May influence future snapshot and migration policy decisions.
