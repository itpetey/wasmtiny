## Why

`wasmtiny` currently treats guest execution as a synchronous run-to-completion call. Selium needs the runtime to pause and resume guest execution safely so async host operations, draining, and future migration do not depend on engine-specific workarounds.

## What Changes

- Add cooperative runtime safepoints that let a running instance suspend at well-defined boundaries.
- Add a resumable instance state model so suspended execution can later continue from the same guest-visible state.
- Add a hostcall suspension path for operations that cannot complete synchronously.
- Require unsupported suspension states to fail explicitly instead of silently falling back to blocking behaviour.

## Capabilities

### New Capabilities
- `runtime-safepoints`: Cooperative suspension and resumption of guest execution at explicit runtime safepoints.

### Modified Capabilities

## Impact

- Affects interpreter execution, JIT entry wrappers, and runtime instance state management.
- Introduces new public runtime APIs for suspend/resume lifecycle.
- Establishes the execution model required for snapshot/restore and async host integration.
