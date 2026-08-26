## ADDED Requirements

### Requirement: Optional Platform Wake Emission on Shared Notify
When compiled in, `memory.atomic.notify` executed on a shared-range
address SHALL additionally emit the host platform's wake primitive
(Linux `futex(FUTEX_WAKE)`, Windows `WakeByAddress`, FreeBSD `_umtx_op`
wake) for the region's host mapping address. Emission SHALL be a side
effect only: the instruction's return value SHALL continue to count
only WASM waiters woken, per the threads proposal. Enablement SHALL be
a build-time decision; when not compiled in, notify SHALL behave
exactly as registry-only.
Platforms whose kernel rejects wait-word primitives on shared mappings
(e.g. macOS) SHALL NOT compile emission in.

#### Scenario: Host OS waiter woken as side effect
- **WHEN** emission is compiled in and a host thread is parked in the
  platform wait-word primitive on the region's host mapping
- **THEN** a guest `memory.atomic.notify` on the corresponding shared
  address SHALL wake it via the platform primitive
- **NOTE** on address-keyed platforms (Windows, FreeBSD) this holds for
  the engine's own mapping; only inode-keyed wakes (Linux `FUTEX_WAKE`
  on `MAP_SHARED`) reach other mappings of the same shm object

#### Scenario: Return value unaffected by emission
- **WHEN** emission is compiled in and one WASM waiter plus any number of
  host OS waiters are woken by a notify with sufficient count
- **THEN** the instruction SHALL return 1 (WASM waiters only)

#### Scenario: Disabled emission is registry-only
- **WHEN** emission is not compiled in (feature disabled or unsupported
  platform)
- **THEN** notify SHALL wake only waiters in the region registry and
  SHALL NOT invoke any platform wake primitive
