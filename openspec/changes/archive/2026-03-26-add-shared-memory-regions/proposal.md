## Why

Selium currently works around runtime limitations by treating shared data as host-managed regions plus extra hostcalls. `wasmtiny` should provide first-class shared memory so guests can exchange large payloads without forced copy-based reads on the hot path.

## What Changes

- Add runtime-managed shared memory regions that can be created independently of a guest's private linear memory.
- Add attach/detach semantics so multiple instances can access the same shared bytes safely.
- Add explicit coherence and bounds guarantees for shared memory mappings.
- Add failure semantics for invalid attachments, overlapping mappings, and use-after-detach conditions.

## Capabilities

### New Capabilities
- `shared-memory-regions`: Runtime-managed shared memory regions with attachable guest mappings and zero-copy visibility semantics.

### Modified Capabilities

## Impact

- Affects memory subsystem design, instance lifecycle, and JIT/interpreter memory access plumbing.
- Introduces new runtime APIs for allocating, attaching, detaching, and inspecting shared regions.
- Reduces the need for host-mediated copy paths in higher-level Selium I/O flows.
