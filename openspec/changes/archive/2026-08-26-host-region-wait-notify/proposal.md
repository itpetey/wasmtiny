# Proposal: Host-Facing Region Wait/Notify

## Why

wasmtiny already runs the threads-proposal wake machinery: a guest's
`memory.atomic.notify` on a shared range pokes a per-region condvar
registry keyed by region offset, and regions are `mmap(MAP_SHARED)`-backed
so host and guest see identical physical pages. But the wait side of that
registry is crate-private (`Memory::wait_on`, `SharedRegion::waiters_arc`,
`SharedMemoryRegistry::get_region` are all `pub(crate)`). Embedders —
Selium's kernel network proxies are the driving case — cannot block on a
region word from outside the engine, so today a guest notify has nothing
host-side to wake. The downstream `shared-page-fastpath` change in Selium
is gated on exactly this API.

## What Changes

- **Public host wait/notify API on shared regions** (Stage 1): embedders
  can register a waiter on `(region_id, offset)`, wait with a timeout,
  and notify waiters — the same per-region registry the interpreter's
  `memory.atomic.notify`/`wait32` already use, so guest and host waits
  interoperate on one mechanism.
- **Lost-wake-safe contract**: waiter registration is explicit and
  separable from waiting, so embedders can do
  register → re-check the shared word → wait. The API documents this
  idiom; a notify with no registered waiter remains a truthful
  zero-woken result.
- **Capability advertisement**: a query reporting the engine's host-wait
  support level (registry-only vs. registry + platform wake emission),
  so embedders detect rather than configure.
- **Optional platform wake emission** (Stage 2, off by default): when
  compiled in and enabled, `memory.atomic.notify` on a shared range also
  emits the host platform's wake primitive (Linux `futex`, macOS
  `__ulock_wake`, Windows `WakeByAddress`, FreeBSD `_umtx_op`) on the
  region's host mapping address, so embedder threads sleeping in the
  matching OS wait-word primitive wake without touching the registry.
  Guest-visible semantics are unchanged: the instruction's return value
  still counts only WASM waiters woken, per the threads proposal.

## Capabilities

### New Capabilities

(None.)

### Modified Capabilities

- `shared-memory-regions`: public host wait/notify API, waiter
  registration contract, capability query
- `wasm-threads`: optional platform wake emission as a side effect of
  `memory.atomic.notify` on shared ranges (guest-visible semantics
  unchanged)

## Impact

- `src/runtime/shared_memory.rs`: visibility and API additions on
  `SharedMemoryRegistry`/`SharedRegion`; no change to allocation,
  attach, or mapping behavior.
- `src/memory.rs`: interpreter wait/notify paths refactored to sit on
  the public registry API (one mechanism, no duplication).
- `src/interpreter/exec.rs`: notify path optionally emits the platform
  wake; behind `cfg` + a runtime enable flag.
- No guest-visible behavior change in Stage 1; no validator changes.
- Fence check (per project intent): this adds a primitive to the
  engine's existing shared-memory scope — no JIT/AOT, no WASI, no
  snapshotting, no policy. It is exactly "append-only multi-memory
  attach" completing its wake story.
