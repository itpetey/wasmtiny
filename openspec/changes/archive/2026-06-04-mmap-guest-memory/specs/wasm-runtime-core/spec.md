## MODIFIED Requirements

### Requirement: Memory access
The runtime SHALL provide safe read/write access to linear memory with bounds checking. Memory access SHALL include both owned pages and mapped shared region pages. Writes to read-only shared pages SHALL trap before reaching memory.

#### Scenario: Out of bounds memory access
- **WHEN** a WASM module attempts to read memory at an offset beyond allocation
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Write to read-only shared region
- **WHEN** a WASM module attempts to write to a shared region page mapped with `PROT_READ`
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Read from mapped shared region
- **WHEN** a WASM module reads memory at an offset within a mapped shared region
- **THEN** the read SHALL succeed and return the shared memory contents

## ADDED Requirements

### Requirement: Mmap-Backed Memory
Guest linear memory SHALL be backed by an `mmap`-based allocation rather than `Vec<u8>`, with a pre-reserved virtual address range supporting growth via `mprotect`.

#### Scenario: Memory created with mmap backing
- **WHEN** a `Memory` is created with `min_pages = 1`
- **THEN** the underlying storage SHALL be an `mmap`'d region with the full maximum virtual address range reserved

#### Scenario: Memory growth extends accessible range
- **WHEN** `Memory::grow(1)` is called
- **THEN** the additional pages SHALL be made accessible via `mprotect` without reallocation

### Requirement: Shared Page Tracking
The `Memory` struct SHALL track which page ranges are owned vs. mapped from shared regions, including the region identifier and protection level for each shared range.

#### Scenario: Shared range queryable
- **WHEN** a guest has attached shared regions
- **THEN** the `Memory` SHALL report the page offsets, region IDs, and protection levels of all attached regions
