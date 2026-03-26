## Why

Selium wants to drain runtimes safely for upgrades and move guest work between runtime processes without losing in-flight state. `wasmtiny` needs a runtime-native snapshot and restore model so migration does not depend on ad hoc application-level reconstruction.

## What Changes

- Add an API to capture an instance snapshot at a safe suspension boundary.
- Add an API to restore a compatible snapshot into a new runtime instance.
- Serialize canonical guest state rather than transient native JIT artefacts.
- Define failure semantics for unsupported resources or incompatible restore targets.

## Capabilities

### New Capabilities
- `instance-snapshots`: Capture, serialize, restore, and resume guest instances from canonical runtime state.

### Modified Capabilities

## Impact

- Depends on a resumable execution model and safe suspension boundaries.
- Affects instance state ownership, JIT cache policy, and memory/global/table persistence.
- Enables future runtime draining and migration workflows in Selium.
