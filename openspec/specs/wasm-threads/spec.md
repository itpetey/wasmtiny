## ADDED Requirements

### Requirement: Shared linear memory support
The runtime SHALL support linear memory with the `shared` attribute as specified in the threads proposal. A shared memory declaration without a maximum SHALL be rejected at validation.

#### Scenario: Memory with shared attribute is validated
- **GIVEN** a WebAssembly module declaring memory with the `shared` attribute and a maximum
- **WHEN** the module is validated
- **THEN** validation SHALL succeed

#### Scenario: Shared memory without maximum is rejected
- **GIVEN** a WebAssembly module declaring `shared` memory with no maximum
- **WHEN** the module is validated
- **THEN** validation SHALL fail

#### Scenario: Atomic operations require shared memory
- **GIVEN** an atomic instruction operating on non-shared memory
- **WHEN** the module is validated
- **THEN** validation SHALL reject the module

### Requirement: Atomic wait operation
The runtime SHALL implement `memory.atomic.wait32` and `memory.atomic.wait64` instructions per the threads proposal: operands SHALL be popped in spec order (timeout, then expected, then address), `wait32`'s expected operand SHALL be i32, and the timeout operand SHALL be interpreted as **nanoseconds** (with -1 meaning wait forever) without overflow for any i64 value. The waiting thread SHALL NOT hold the guest memory lock (or instance lock) while parked, so that `memory.atomic.notify` on the same memory can proceed.

#### Scenario: atomic.wait returns not-equal
- **GIVEN** a shared memory location whose value differs from the expected operand
- **WHEN** atomic.wait is executed
- **THEN** it SHALL return 1 (not equal) immediately

#### Scenario: atomic.wait returns woken
- **GIVEN** a thread parked in atomic.wait on a shared memory location
- **WHEN** another context calls atomic.notify on that location on the same memory
- **THEN** the waiter SHALL wake and return 0 (woken) without waiting for the timeout

#### Scenario: atomic.wait times out
- **GIVEN** a shared memory location with no notify activity
- **WHEN** atomic.wait is called with a finite nanosecond timeout
- **THEN** it SHALL return 2 (timed out) after approximately that duration

#### Scenario: atomic.wait with large timeout does not overflow
- **GIVEN** any i64 timeout value (including very large positive values)
- **WHEN** atomic.wait is executed
- **THEN** the timeout SHALL be honoured without arithmetic overflow, panic, or wraparound

### Requirement: Atomic notify operation
The runtime SHALL implement `memory.atomic.notify` per the threads proposal: operands SHALL be popped in spec order (count, then address), up to `count` **distinct** waiters SHALL be woken, and the instruction SHALL return the actual number of waiters woken.

#### Scenario: atomic.notify wakes distinct waiters
- **GIVEN** three threads waiting on a memory location
- **WHEN** atomic.notify is called with count 2
- **THEN** exactly two distinct waiters SHALL be woken and the instruction SHALL return 2

#### Scenario: atomic.notify with no waiters returns zero
- **GIVEN** no waiters on a memory location
- **WHEN** atomic.notify is called
- **THEN** it SHALL return 0

### Requirement: Sequential consistency
All atomic operations SHALL use sequential consistency (SeqCst) memory ordering.

#### Scenario: Multiple threads see consistent state
- **GIVEN** multiple threads performing atomic operations
- **WHEN** operations complete
- **THEN** all threads SHALL agree on the order of operations

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
- **WHEN** emission is enabled and a host thread is parked in the
  platform wait-word primitive on the region's host mapping
- **THEN** a guest `memory.atomic.notify` on the corresponding shared
  address SHALL wake it via the platform primitive
- **NOTE** on address-keyed platforms (Windows, FreeBSD) this holds for
  the engine's own mapping; only inode-keyed wakes (Linux `FUTEX_WAKE`
  on `MAP_SHARED`) reach other mappings of the same shm object

#### Scenario: Return value unaffected by emission
- **WHEN** emission is enabled and one WASM waiter plus any number of
  host OS waiters are woken by a notify with sufficient count
- **THEN** the instruction SHALL return 1 (WASM waiters only)

#### Scenario: Disabled emission is registry-only
- **WHEN** emission is not compiled in (feature disabled or unsupported
  platform)
- **THEN** notify SHALL wake only waiters in the region registry and
  SHALL NOT invoke any platform wake primitive
