## ADDED Requirements

### Requirement: Shared linear memory support
The runtime SHALL support linear memory with the `shared` attribute as specified in the threads proposal.

#### Scenario: Memory with shared attribute is validated
- **GIVEN** a WebAssembly module declaring memory with `shared` attribute
- **WHEN** the module is validated
- **THEN** validation SHALL succeed

#### Scenario: Atomic operations require shared memory
- **GIVEN** an atomic instruction operating on non-shared memory
- **WHEN** the module is executed
- **THEN** the runtime SHALL trap with an appropriate error

### Requirement: Atomic wait operation
The runtime SHALL implement `memory.atomic.wait32` and `memory.atomic.wait64` instructions.

#### Scenario: atomic.wait returns ok
- **GIVEN** a shared memory location with value X
- **WHEN** atomic.wait is called with expected value X
- **THEN** it SHALL return 0 (ok) immediately

#### Scenario: atomic.wait returns woken
- **GIVEN** a shared memory location with value X different from expected
- **AND** another thread calls atomic.notify
- **WHEN** atomic.wait is called
- **THEN** it SHALL return 1 (woken) when notified

#### Scenario: atomic.wait times out
- **GIVEN** a shared memory location
- **WHEN** atomic.wait is called with a timeout that expires
- **THEN** it SHALL return 2 (timed out)

### Requirement: Atomic notify operation
The runtime SHALL implement `memory.atomic.notify` instruction.

#### Scenario: atomic.notify wakes waiters
- **GIVEN** one or more threads waiting on a memory location
- **WHEN** atomic.notify is called on that location
- **THEN** the specified number of waiters SHALL be woken

#### Scenario: atomic.notify returns count
- **GIVEN** waiters on a memory location
- **WHEN** atomic.notify is called
- **THEN** it SHALL return the number of waiters woken

### Requirement: Sequential consistency
All atomic operations SHALL use sequential consistency (SeqCst) memory ordering.

#### Scenario: Multiple threads see consistent state
- **GIVEN** multiple threads performing atomic operations
- **WHEN** operations complete
- **THEN** all threads SHALL agree on the order of operations
