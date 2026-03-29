## ADDED Requirements

### Requirement: AOT compiler handles atomic operations
The AOT compiler SHALL correctly compile all atomic WASM instructions to native code.

#### Scenario: AOT atomic load produces correct result
- **GIVEN** a WebAssembly module with i32.atomic.load
- **WHEN** the module is compiled to AOT
- **AND** executed
- **THEN** it SHALL return the correct value from shared memory

#### Scenario: AOT atomic store produces correct result
- **GIVEN** a WebAssembly module with i32.atomic.store
- **WHEN** the module is compiled to AOT
- **AND** executed
- **THEN** it SHALL write the correct value to shared memory

#### Scenario: AOT atomic rmw produces correct result
- **GIVEN** a WebAssembly module with atomic.rmw.add
- **WHEN** the module is compiled to AOT
- **AND** executed
- **THEN** the value SHALL be atomically modified

#### Scenario: AOT atomic.wait blocks correctly
- **GIVEN** a WebAssembly module with memory.atomic.wait32
- **WHEN** the module is compiled to AOT
- **AND** executed with non-matching value
- **THEN** it SHALL block the thread until woken or timed out

#### Scenario: AOT atomic.notify wakes correctly
- **GIVEN** a WebAssembly module with memory.atomic.notify
- **WHEN** the module is compiled to AOT
- **AND** executed
- **THEN** it SHALL wake the specified number of waiters
