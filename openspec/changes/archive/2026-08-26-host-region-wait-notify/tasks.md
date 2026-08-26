# Tasks: Host-Facing Region Wait/Notify

## 1. Public registry API (Stage 1)

- [x] 1.1 Add `SharedMemoryRegistry::register_region_waiter(region_id, offset) -> Result<Arc<RegionWaiter>>` and `RegionWaiter::wait(timeout) -> Result<WakeOutcome>` (`Woken | TimedOut`), reusing the existing per-region `SharedWaiter` mechanism
- [x] 1.2 Add `SharedMemoryRegistry::notify_region(region_id, offset, count) -> Result<u32>` for host-initiated wakes
- [x] 1.3 `RegionWaiter` deregisters on drop; document the register → re-check → wait idiom in rustdoc
- [x] 1.4 Refactor `Memory::wait_on`/`Memory::notify` shared-range paths onto the public registry functions (single mechanism; preserve `wasm-threads` semantics incl. not holding the guest memory lock while parked)

## 2. Capability advertisement

- [x] 2.1 Add `HostWaitSupport` (`RegistryOnly | RegistryAndOsWake`) and `SharedMemoryRegistry::host_wait_support()`
- [x] 2.2 Report `RegistryAndOsWake` only when the emission code is compiled in (build-time `platform-wake-emission` cargo feature on a supported OS; no runtime toggle)

## 3. Optional platform wake emission (Stage 2)

- [x] 3.1 Add per-OS wake emission behind `cfg(target_os)`: Linux `futex(FUTEX_WAKE)`, Windows `WakeByAddress`, FreeBSD `_umtx_op` wake; no-op elsewhere (macOS excluded — Darwin rejects wait-word primitives on `MAP_SHARED` memory; backend retained)
- [x] 3.2 Interpreter notify path on shared ranges emits the platform wake for `region.ptr() + offset` when compiled in; instruction return value unchanged (counts only WASM waiters)
- [x] 3.3 Enablement is the `platform-wake-emission` cargo feature, build-time only; documented as embedder-detected via `host_wait_support()`

## 4. Verification

- [x] 4.1 Test: host thread registered on `(region, offset)` wakes when a guest executes `memory.atomic.notify` on that address
- [x] 4.2 Test: register → re-check → wait idiom — notify landing between word check and wait is not lost (loop many iterations)
- [x] 4.3 Test: guest `memory.atomic.wait32` on a shared range wakes from host-initiated `notify_region`
- [x] 4.4 Test: notify return value counts only WASM waiters even when host waiters are also woken (Stage 2 enabled)
- [x] 4.5 Per-OS conformance test (Stage 2 platforms): guest notify wakes a host thread parked in the OS wait-word primitive on the same pages; CI gates enablement on this passing
- [x] 4.6 Existing threads-proposal and shared-region test suites stay green
