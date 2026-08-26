# Design: Host-Facing Region Wait/Notify

## Context

`Memory::notify` already routes shared-range addresses into the region's
`SharedWaiter` registry (condvar per offset, `Arc`-shared across
attachments); `Memory::wait_on` does the same for waits. Both are
reachable only from inside the crate. Selium's host proxies currently
sleep in their own registry in another crate, so the wake graph is
broken by visibility, not by mechanism. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- One wait/notify registry per region, usable from both interpreter and
  embedder code
- A lost-wake-safe waiting contract for embedders
- Optional per-OS wake emission without guest-visible semantic change

**Non-Goals:**

- Fair queuing or priority among waiters
- Cross-process waits (embedders are in-process; `shm_open` cross-process
  coordination is out of scope)
- Changing threads-proposal validation or guest-visible wait semantics
- Exposing the registry's internals (lock types, map layout)

## Decisions

### Handle-based registration, not bare wait

The race: embedder reads the ring word (old), guest bumps + notifies
before the embedder registers a waiter, notify returns zero-woken,
embedder then sleeps forever. Bare `wait(region, offset, timeout)`
cannot close this from the outside.

So the API separates registration from waiting:

```
let waiter = registry.register_region_waiter(region_id, offset)?; // Arc<RegionWaiter>
// caller re-checks the shared word here
waiter.wait(timeout)?;           // returns Woken | TimedOut
registry.notify_region(region_id, offset, count)?; // host→host/host→guest wakes
```

Registration inserts into the region's waiter map; `wait` checks the
notified flag under the waiter's mutex before sleeping, and notify sets
that flag under the same mutex — the register → re-check → wait idiom
is then race-free. `RegionWaiter` deregs on drop so the map does not
leak stale entries. Alternative considered (bare `wait_region` +
documented timeout): rejected — a timeout you must not rely on is a
latency bug farm.

### The interpreter sits on the same API

`Memory::wait_on`/`notify` shared-range paths are re-expressed over the
public registry functions so there is exactly one wait mechanism per
region. Guest `wait32` semantics (including "must not hold the guest
memory lock while parked", per `wasm-threads`) are preserved.

### Capability advertisement as a level query

`SharedMemoryRegistry::host_wait_support() -> HostWaitSupport` where
`HostWaitSupport` is `RegistryOnly | RegistryAndOsWake` (the latter only
when the `platform-wake-emission` cargo feature is enabled on a
supported OS). Enablement is a **build-time decision**: there is no
runtime flag, so the level is a process-wide immutable constant —
identical for every registry and store, and safe under multi-tenancy
(no tenant can flip it for another). Embedders branch on it once at
attach; there is no toggle the embedder can set.

### Stage 2 emission is a side effect, never semantics

When `RegistryAndOsWake` is compiled in, the interpreter's notify on a
shared range additionally emits the platform wake for
`region.ptr() + offset`. The instruction's return value still counts
only WASM waiters woken (threads-proposal conformance); host OS waiters
are invisible to the guest. Platform table: Linux `futex(FUTEX_WAKE)`,
Windows `WakeByAddress`, FreeBSD `_umtx_op` wake; macOS is excluded
because Darwin rejects wait-word primitives on `MAP_SHARED` memory;
other platforms compile to no emission. The `platform-wake-emission`
feature is gated per platform on a notify/wait race conformance test
passing in CI.

## Risks / Trade-offs

- `RegionWaiter` handles escaping registration scope → dereg-on-drop
  plus a documented "register cheap, wait often" note; map growth is
  bounded by live waiters.
- macOS `__ulock_*` is undocumented → Stage 2 only, `cfg`-isolated,
  gated by the conformance test; Stage 1 never uses it.
- A missed wake would hang an embedder thread → embedders keep a
  bounded timeout as backstop (Selium's proxies do), and the
  conformance test hammers register/notify/wait races.

## Migration Plan

Purely additive: existing `pub` items unchanged; previously
`pub(crate)` internals gain public wrappers. Selium migrates its
host-side waits onto the registry in its own `shared-page-fastpath`
change.
