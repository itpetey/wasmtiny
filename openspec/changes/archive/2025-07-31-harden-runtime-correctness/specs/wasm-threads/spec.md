## MODIFIED Requirements

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
