## ADDED Requirements

### Requirement: AOT compiler handles memory64 operations
The AOT compiler SHALL correctly compile memory64 operations to native code.

#### Scenario: AOT i64.load produces correct result
- **GIVEN** a WebAssembly module with i64.load
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** it SHALL return the correct 64-bit value

#### Scenario: AOT i64.store produces correct result
- **GIVEN** a WebAssembly module with i64.store
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** it SHALL store the correct value

#### Scenario: AOT memory.size returns i64
- **GIVEN** a memory64 module
- **WHEN** compiled to AOT
- **AND** memory.size is called
- **THEN** it SHALL return i64 page count

#### Scenario: AOT memory.grow accepts i64
- **GIVEN** a memory64 module
- **WHEN** compiled to AOT
- **AND** memory.grow is called
- **THEN** it SHALL accept and return i64

### Requirement: AOT handles memory64 bounds checking
The AOT compiler SHALL generate proper bounds checking for 64-bit addresses.

#### Scenario: AOT out-of-bounds trap
- **GIVEN** a memory64 module accessing beyond bounds
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** it SHALL trap with out-of-bounds error
