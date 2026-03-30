## ADDED Requirements

### Requirement: Atomic load operations
The interpreter SHALL implement atomic load operations for i32, i64, i8, and i16 types.

#### Scenario: i32.atomic.load returns value
- **GIVEN** shared memory with value 0x12345678 at address 0
- **WHEN** i32.atomic.load is executed
- **THEN** it SHALL return 0x12345678

#### Scenario: i64.atomic.load returns value
- **GIVEN** shared memory with value 0x123456789ABCDEF0 at address 0
- **WHEN** i64.atomic.load is executed
- **THEN** it SHALL return 0x123456789ABCDEF0

### Requirement: Atomic store operations
The interpreter SHALL implement atomic store operations for i32, i64, i8, and i16 types.

#### Scenario: i32.atomic.store writes value
- **GIVEN** shared memory
- **WHEN** i32.atomic.store writes 0xDEADBEEF
- **THEN** subsequent i32.atomic.load SHALL return 0xDEADBEEF

### Requirement: Atomic read-modify-write operations
The interpreter SHALL implement atomic rmw operations: add, sub, and, or, xor, xchg, cmpxchg.

#### Scenario: i32.atomic.rmw.add
- **GIVEN** shared memory with value 10
- **WHEN** atomic.rmw.add with operand 5 is executed
- **THEN** the new value SHALL be 15

#### Scenario: i32.atomic.rmw.cmpxchg succeeds
- **GIVEN** shared memory with value 10
- **WHEN** atomic.rmw.cmpxchg with expected 10, replacement 20
- **THEN** it SHALL return 10 (old value) and memory SHALL be 20

#### Scenario: i32.atomic.rmw.cmpxchg fails
- **GIVEN** shared memory with value 10
- **WHEN** atomic.rmw.cmpxchg with expected 15, replacement 20
- **THEN** it SHALL return 10 (old value) and memory SHALL be unchanged

### Requirement: Atomic fence operations
The interpreter SHALL handle atomic fence operations with sequential consistency.

#### Scenario: atomic fence enforces ordering
- **GIVEN** multiple threads with atomic operations
- **WHEN** atomic.fence is executed
- **THEN** it SHALL enforce that all prior atomic operations complete before subsequent ones
