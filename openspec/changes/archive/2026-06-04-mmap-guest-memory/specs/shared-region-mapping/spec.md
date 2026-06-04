## ADDED Requirements

### Requirement: Map Region into Guest Memory
The runtime SHALL map shared memory regions into a guest's linear memory address space so that the guest accesses them via native WASM load, store, and atomic instructions.

#### Scenario: AllocRegion maps pages into guest memory
- **WHEN** `SharedMemoryRegistry::alloc_region(pages, prot)` is called for a guest instance
- **THEN** the runtime SHALL `mmap` a new shared region, extend the guest's memory address space to include those pages at the next available page offset, and return the `(region_id, page_offset)`

#### Scenario: AttachRegion maps existing pages
- **WHEN** `SharedMemoryRegistry::attach_region(region_id, reader_slot, prot)` is called for a guest instance
- **THEN** the runtime SHALL map the existing region's pages into the guest's address space at the next available page offset and return the `page_offset`

### Requirement: Per-Page Protection on Attach
When `attach_region` specifies a `reader_slot`, the runtime SHALL apply per-page `mprotect` so only the designated reader cursor page is writable.

#### Scenario: Reader slot page is writable
- **WHEN** a consumer attaches with `reader_slot: Some(2)`
- **THEN** page 2 of the mapped region SHALL be `PROT_READ | PROT_WRITE` and all other pages SHALL be `PROT_READ`

### Requirement: Region Detach Unmaps Pages
When `free_region` is called, the runtime SHALL `munmap` the region's pages from the guest's address space and restore the range to `PROT_NONE`.

#### Scenario: Detach unmaps pages
- **WHEN** a guest calls `free_region` on an attached region
- **THEN** the pages SHALL be unmapped from guest memory and the page range SHALL revert to inaccessible

### Requirement: Kernel-Serialized Detach Safety
The runtime SHALL rely on the operating system kernel to serialize `munmap` with in-flight memory accesses, eliminating the need for a software `RwLock` gate on shared memory operations.

#### Scenario: Concurrent access during detach
- **WHEN** one thread accesses a shared page while another thread detaches the region
- **THEN** the kernel SHALL ensure the access either completes before the unmap or generates `SIGSEGV` that the runtime translates to a trap

### Requirement: Simplified Registry Without Read/Write Path
`SharedMemoryRegistry` SHALL manage region lifecycle (create, destroy, attach, detach) without providing byte-level read/write methods.

#### Scenario: Registry has no read method
- **WHEN** inspecting the `SharedMemoryRegistry` public API
- **THEN** there SHALL be no `read` or `write` methods; all data access SHALL go through direct memory access by the guest
