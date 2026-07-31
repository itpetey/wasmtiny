## MODIFIED Requirements

### Requirement: Shared Region Allocation
The runtime SHALL allow callers to allocate shared memory regions that are mapped directly into guest linear memory via `mmap`, accessible through native WASM load/store/atomic instructions.

#### Scenario: Create shared region
- **WHEN** a caller requests a shared region with a valid size and alignment
- **THEN** the runtime SHALL create an `mmap`-backed shared region, map it into the guest's linear memory at the next available page offset, and return `(region_id, page_offset)`

### Requirement: Explicit Region Attachment
The runtime SHALL require instances to attach a shared region explicitly before accessing it. Attachment SHALL map the region's pages into the guest's linear memory.

#### Scenario: Attach region to instance
- **WHEN** a caller attaches a valid shared region to an instance
- **THEN** the runtime SHALL map the region's pages into the guest's linear memory at the next available page offset

### Requirement: Cross-Instance Visibility
The runtime SHALL make writes to an attached shared region visible to other instances attached to the same region via direct memory mapping without software copy paths.

#### Scenario: Observe shared write
- **WHEN** one attached instance writes to a shared region via `i32.store`
- **THEN** another attached instance SHALL be able to observe the updated bytes via `i32.load` without requiring the runtime to perform an implicit copy

### Requirement: Explicit Detach Failure Semantics
The runtime SHALL reject accesses to unmapped shared pages via bounds checking. Access rejection SHALL be enforced by the runtime's software checks (not by relying on OS signal delivery), so trap behaviour does not depend on process-wide signal-handler state.

#### Scenario: Access after detach
- **WHEN** an instance attempts to access a shared region after its mapping has been detached via `free_region`
- **THEN** the access SHALL trap because the pages are no longer mapped in the guest's address space

## REMOVED Requirements

### Requirement: Shared Memory Mapping Byte-Level Read/Write
**Reason**: With regions mapped into guest linear memory, the `ResolvedSharedMemoryMapping` byte-copy read/write path is replaced by direct memory access. The `SharedMemoryMapping`, `SharedMemoryMappingId`, `ResolvedSharedMemoryMapping`, and `SharedMemoryMappingState` types are removed.
**Migration**: All shared memory access uses native WASM load/store at the page offset returned by `alloc_region`/`attach_region`.

## ADDED Requirements

### Requirement: Per-Page Protection on Attach
When attaching a shared region, the runtime SHALL accept an optional `reader_slot` parameter and apply per-page `mprotect` so only the designated reader cursor page is writable for the calling instance.

#### Scenario: Consumer attach with reader slot
- **WHEN** an instance attaches to a shared region with `reader_slot: Some(1)`
- **THEN** page 1 of the mapped region SHALL be `PROT_READ | PROT_WRITE` and all other pages SHALL be `PROT_READ`

#### Scenario: Producer attach without reader slot
- **WHEN** an instance attaches to a shared region with `reader_slot: None`
- **THEN** all pages SHALL be mapped `PROT_READ | PROT_WRITE`

### Requirement: Region Lifecycle Without Software Gate
The registry SHALL manage region and attachment lifecycle without a software read/write gate (`RwLock`), relying on the OS kernel to serialize `munmap` with in-flight memory accesses.

#### Scenario: Detach during concurrent access
- **WHEN** `free_region` unmaps pages while another thread accesses them
- **THEN** the kernel SHALL ensure accesses either complete before unmap or generate a signal the runtime handles

### Requirement: Overflow-safe region arithmetic
All region size computations (alignment round-up, capacity checks) SHALL use checked or sufficiently wide arithmetic, and region sizes SHALL be capped at a defined maximum, so that no host-supplied size can overflow, panic in debug builds, or wrap to an undersized allocation in release builds.

#### Scenario: Oversized region request fails cleanly
- **WHEN** a caller requests a shared region whose size is near or above u32::MAX
- **THEN** allocation SHALL return an explicit error without panic, wrap, or undersized mapping

### Requirement: Host I/O bounds check integrity
Host-side region read/write operations SHALL validate `offset` and `length` with overflow-proof comparisons before any pointer arithmetic or `unsafe` copy.

#### Scenario: Overflowing offset rejected
- **WHEN** a host caller reads or writes a region at an offset whose `offset + length` would overflow `usize`
- **THEN** the operation SHALL return an explicit bounds error and perform no memory access

### Requirement: Unique region naming
Shared region backing-object names SHALL be unique across concurrent processes (incorporating process identity and/or entropy), so that unrelated wasmtiny processes cannot collide.

#### Scenario: Concurrent processes allocate regions
- **WHEN** two wasmtiny processes allocate shared regions concurrently
- **THEN** neither allocation SHALL fail due to a name collision with the other process
