## ADDED Requirements

### Requirement: Mmap-Backed Memory Allocation
Guest linear memory SHALL be backed by an `mmap`-based allocation with a pre-reserved virtual address range spanning the maximum allowed pages, rather than a `Vec<u8>`.

#### Scenario: Memory creation reserves full VA range
- **WHEN** a guest `Memory` is created with `min_pages = 1` and `max_pages = 65536`
- **THEN** the runtime SHALL `mmap` 4GB of virtual address space with `PROT_NONE` and `mprotect` the first 64KB to `PROT_READ | PROT_WRITE`

#### Scenario: Memory growth via mprotect
- **WHEN** `Memory::grow(1)` is called
- **THEN** the runtime SHALL `mprotect` the next 64KB of the reserved range to `PROT_READ | PROT_WRITE` and return the previous page count

### Requirement: Memory Bounds Cover Shared Pages
`Memory::len_bytes()` and all bounds-checking logic SHALL include both owned and mapped shared pages in the valid address range.

#### Scenario: Load from shared page passes bounds check
- **WHEN** a guest executes `i32.load` at an address within a mapped shared region
- **THEN** the bounds check SHALL pass because the address falls within the extended memory range

#### Scenario: Load beyond all pages fails bounds check
- **WHEN** a guest executes `i32.load` at an address beyond the maximum of owned and shared pages
- **THEN** the bounds check SHALL fail with `MemoryOutOfBounds`

### Requirement: Write Protection Check
`Memory::write` SHALL validate that the target address range does not overlap any read-only shared region before performing the write, producing a WASM trap if it does.

#### Scenario: Write to read-only shared page traps
- **WHEN** a guest executes `i32.store` targeting a read-only shared page
- **THEN** the runtime SHALL trap with `MemoryOutOfBounds` before the store reaches memory

#### Scenario: Write to writable shared page succeeds
- **WHEN** a guest executes `i32.store` targeting a writable shared page
- **THEN** the write SHALL succeed and the value SHALL be visible to other instances attached to the same region

### Requirement: Owned vs Shared Page Tracking
The `Memory` struct SHALL maintain a list of shared page ranges distinct from owned pages, recording the `(page_offset, region_id, len, prot)` for each attached region.

#### Scenario: Shared range recorded on attach
- **WHEN** a shared region is attached to a guest instance
- **THEN** the range SHALL be added to `Memory.shared_ranges` with the region identifier and protection level

#### Scenario: Shared range removed on detach
- **WHEN** a shared region is detached from a guest instance
- **THEN** the corresponding entry SHALL be removed from `Memory.shared_ranges`
