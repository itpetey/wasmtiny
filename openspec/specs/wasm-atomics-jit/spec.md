## ADDED Requirements

### Requirement: JIT compiler emits atomic operations
The JIT compiler SHALL correctly emit native atomic operations for all atomic WASM instructions.

#### Scenario: JIT atomic load generates lock-prefixed mov
- **GIVEN** a WebAssembly module with i32.atomic.load
- **WHEN** the module is compiled with JIT
- **AND** executed
- **THEN** it SHALL return the correct value from shared memory

#### Scenario: JIT atomic store generates lock-prefixed mov
- **GIVEN** a WebAssembly module with i32.atomic.store
- **WHEN** the module is compiled with JIT
- **AND** executed
- **THEN** it SHALL write the correct value to shared memory

#### Scenario: JIT atomic rmw generates lock-prefixed operation
- **GIVEN** a WebAssembly module with atomic.rmw.add
- **WHEN** the module is compiled with JIT
- **AND** executed
- **THEN** the value SHALL be atomically modified

#### Scenario: JIT atomic.wait generates park operation
- **GIVEN** a WebAssembly module with memory.atomic.wait32
- **WHEN** the module is compiled with JIT
- **AND** executed with non-matching value
- **THEN** it SHALL block the thread until woken or timed out

#### Scenario: JIT atomic.notify generates unpark operation
- **GIVEN** a WebAssembly module with memory.atomic.notify
- **WHEN** the module is compiled with JIT
- **AND** executed
- **THEN** it SHALL wake the specified number of waiters
