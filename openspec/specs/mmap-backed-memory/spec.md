## ADDED Requirements

### Requirement: Mmap-Backed Memory Allocation
Guest linear memory SHALL be backed by an `mmap`-based allocation with a pre-reserved virtual address range spanning the maximum allowed pages, rather than a `Vec<u8>`. This support is Unix-only; the crate SHALL document this platform constraint and gate the implementation accordingly.

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

### Requirement: Growth isolation from shared mappings
Owned-page growth via `mprotect` SHALL never overlap or alias mapped shared regions. Shared region placement within the reserved virtual address range SHALL be decoupled from owned-page growth (e.g. shared mappings placed from the top of the reserved range downwards while owned pages grow upwards).

#### Scenario: Grow after attach yields zeroed owned pages
- **GIVEN** a guest memory with an attached shared region
- **WHEN** `memory.grow` executes
- **THEN** the new pages SHALL be zero-filled owned pages, the shared mapping SHALL remain intact at its offset, and the shared region's protection SHALL be unchanged

#### Scenario: Detach after grow leaves no accessible hole
- **GIVEN** a guest memory that grew after attaching a shared region
- **WHEN** the shared region is detached
- **THEN** every address in `[0, len_bytes)` that passes the bounds check SHALL be backed by an accessible owned or shared page — no `PROT_NONE` hole SHALL exist inside the valid range

### Requirement: Fallible memory allocation
Memory creation and growth SHALL surface OS allocation failures (`mmap`/`mprotect` errors, virtual-address exhaustion) as `Err` results and SHALL never panic or abort the host process.

#### Scenario: Reservation failure returns error
- **WHEN** the OS cannot satisfy the virtual-address reservation for a new guest memory
- **THEN** memory creation SHALL return an `Err` and the host process SHALL remain alive

### Requirement: Defined memory clone semantics
Cloning a `Memory` (where cloning remains exposed at all) SHALL produce a memory whose shared-range metadata is consistent with its actual mappings: a clone either re-establishes shared mappings or carries no shared-range state.

#### Scenario: Clone has no dangling shared ranges
- **WHEN** a `Memory` with attached shared regions is cloned without re-attaching those regions
- **THEN** the clone's bounds and metadata SHALL NOT claim addresses as valid shared pages that are not mapped in the clone
