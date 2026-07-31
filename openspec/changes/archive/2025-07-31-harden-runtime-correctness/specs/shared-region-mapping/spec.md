## ADDED Requirements

### Requirement: Duplicate attach rejection
Attaching a region that is already attached to a given memory SHALL be rejected with an explicit error, keeping `attachment_count` consistent with actual mappings.

#### Scenario: Second attach of same region rejected
- **WHEN** a caller attaches a region to a memory that already has that region attached
- **THEN** the operation SHALL return an explicit error and SHALL NOT create a second mapping or increment the attachment count

### Requirement: Partial-failure cleanup
If any step of mapping a shared region fails after earlier steps succeeded (e.g. protection setup after the address-space mapping), all effects of the partial operation SHALL be rolled back.

#### Scenario: Failed protection setup unmaps region
- **WHEN** per-page protection setup fails during attach after the region was mapped
- **THEN** the mapping SHALL be torn down, no shared-range entry SHALL be recorded, and the attachment count SHALL be unchanged

## MODIFIED Requirements

### Requirement: Map Region into Guest Memory
The runtime SHALL map shared memory regions into a guest's linear memory address space so that the guest accesses them via native WASM load, store, and atomic instructions. Shared mappings SHALL be placed at offsets that owned-page growth can never reach (e.g. descending from the top of the reserved virtual address range), so that `memory.grow` can never alias a shared mapping.

#### Scenario: AllocRegion maps pages into guest memory
- **WHEN** `SharedMemoryRegistry::alloc_region(pages, prot)` is called for a guest instance
- **THEN** the runtime SHALL `mmap` a new shared region, extend the guest's valid memory range to include those pages at an allocated offset outside the owned-growth path, and return the `(region_id, page_offset)`

#### Scenario: AttachRegion maps existing pages
- **WHEN** `SharedMemoryRegistry::attach_region(region_id, reader_slot, prot)` is called for a guest instance
- **THEN** the runtime SHALL map the existing region's pages into the guest's address space at an allocated offset outside the owned-growth path and return the `page_offset`

### Requirement: Kernel-Serialized Detach Safety
The runtime SHALL rely on the operating system kernel to serialize `munmap` with in-flight memory accesses, eliminating the need for a software `RwLock` gate on shared memory operations. Guest-observable trap behaviour for invalid accesses SHALL be guaranteed by the runtime's software bounds checks; the runtime SHALL NOT require a process-wide POSIX signal handler for correct trap semantics.

#### Scenario: Concurrent access during detach
- **WHEN** one thread accesses a shared page while another thread detaches the region
- **THEN** the access SHALL either complete before the unmap or fail as a trap surfaced through the runtime's own checks

#### Scenario: No signal handler installation
- **WHEN** the runtime is initialised and executes guests
- **THEN** it SHALL NOT install or replace process-wide POSIX signal handlers
