## Context

`wasmtiny` does not currently expose a durable instance image. Yet one of the runtime's strategic goals is to support safe draining and eventual migration, which requires capturing enough semantic guest state to restore execution elsewhere without embedding engine-specific machine frames in the snapshot.

## Goals / Non-Goals

**Goals:**
- Define a canonical snapshot format for guest semantic state.
- Capture snapshots only at runtime-approved safepoints.
- Restore snapshots into a compatible runtime instance and resume execution.
- Fail clearly when a snapshot includes unsupported runtime state.

**Non-Goals:**
- Arbitrary-point machine-frame snapshots.
- Cross-version compatibility with no format/version checks.
- Full distributed migration orchestration in Selium.

## Decisions

### 1. Snapshots are taken only at safepoints
Snapshot capture builds on runtime safepoints so the runtime only serializes stable semantic state, not arbitrary transient execution frames.

### 2. Canonical semantic state is the snapshot payload
Snapshots will contain guest-visible state such as memories, globals, tables, and resumable execution state. Native JIT code and raw host pointers are reconstructed after restore rather than embedded in the snapshot.

### 3. Snapshot format is versioned and compatibility-checked
Restore must validate module identity, runtime format version, and any other compatibility markers before execution resumes.

### 4. Unsupported resources fail loudly
If a snapshot references runtime state that cannot be serialized safely, capture must fail with an explicit reason rather than emitting a partial or misleading image.

## Risks / Trade-offs

- [Large snapshot payloads] -> Keep the format explicit and incremental-friendly, then optimise once correctness is proven.
- [JIT mismatch on restore] -> Treat JIT code as rebuildable cache state, never the canonical snapshot payload.
- [Attachment/resource ambiguity] -> Make unsupported resources explicit in the capture contract.
- [Format lock-in] -> Version the snapshot format from the start.
