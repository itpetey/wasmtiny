## ADDED Requirements

### Requirement: Memory64 type in module format
The runtime SHALL support the memory64 type in WebAssembly modules as specified in the proposal.

#### Scenario: Module with memory64 is parsed
- **GIVEN** a WebAssembly module declaring `memory 64 128`
- **WHEN** the module is parsed
- **THEN** it SHALL be recognized as memory64

#### Scenario: Memory64 with limits
- **GIVEN** a WebAssembly module declaring `memory 64 128 65536`
- **WHEN** the module is validated
- **THEN** it SHALL have initial 128 pages, max 65536 pages

#### Scenario: Memory64 page size
- **GIVEN** a memory64 with 1024 pages
- **WHEN** calculating total bytes
- **THEN** it SHALL be 1024 * 64KiB = 64MiB

### Requirement: Memory64 addressing
The runtime SHALL support 64-bit addressing for memory64.

#### Scenario: Load from 64-bit address
- **GIVEN** a memory64 with data at address 0x1_0000_0000
- **WHEN** i64.load is executed with that address
- **THEN** it SHALL return the correct value

#### Scenario: Store to 64-bit address
- **GIVEN** a memory64
- **WHEN** i64.store is executed at address 0x1_0000_0000
- **THEN** the value SHALL be stored at that address

#### Scenario: Memory64 address space limit
- **GIVEN** a memory64 with max 256 pages
- **WHEN** accessing address 256 * 65536
- **THEN** the runtime SHALL trap (out of bounds)

### Requirement: Memory64 size and grow
The runtime SHALL implement memory.size and memory.grow for memory64.

#### Scenario: memory.size returns page count
- **GIVEN** a memory64 with 100 pages
- **WHEN** memory.size is called
- **THEN** it SHALL return 100 as i64

#### Scenario: memory.grow increases memory
- **GIVEN** a memory64 with 100 pages
- **WHEN** memory.grow is called with 50
- **THEN** memory SHALL grow to 150 pages
- **AND** memory.grow SHALL return 100

#### Scenario: memory.grow at limit
- **GIVEN** a memory64 with max 100 pages, currently at 100
- **WHEN** memory.grow is called
- **THEN** it SHALL return -1 (failure)

### Requirement: Memory64 with shared attribute
The runtime SHALL support memory64 with the shared attribute.

#### Scenario: Shared memory64
- **GIVEN** a WebAssembly module declaring `memory 64 shared 128 256`
- **WHEN** the module is validated
- **THEN** it SHALL be recognized as shared memory64

### Requirement: Memory64 data segment instructions
The runtime SHALL implement memory.init and data.drop for memory64.

#### Scenario: Initialize memory from data segment
- **GIVEN** a memory64 with a data segment
- **WHEN** memory.init is executed
- **THEN** the data SHALL be copied to the specified offset

#### Scenario: Drop data segment
- **GIVEN** a memory64 with an active data segment
- **WHEN** data.drop is executed
- **THEN** the data segment SHALL be dropped
