## 1. Memory Backing: Vec<u8> to mmap

- [x] 1.1 Replace `Memory.data: Vec<u8>` with `ptr: *mut u8`, `len: usize`, `capacity: usize`
- [x] 1.2 Implement `Memory::new` using `mmap` with `PROT_NONE` for full capacity and `mprotect` for initial pages
- [x] 1.3 Implement `Memory::grow` using `mprotect` to extend accessible range (no reallocation)
- [x] 1.4 Implement `Memory::read` and `Memory::write` using `ptr::copy_nonoverlapping` instead of slice indexing
- [x] 1.5 Implement `Memory::len_bytes` returning the full extended size (owned + shared)
- [x] 1.6 Add `SharedRange` struct and `shared_ranges: Vec<SharedRange>` to `Memory`
- [x] 1.7 Implement `Memory::check_writable` that scans `shared_ranges` for read-only overlaps
- [x] 1.8 Implement `Drop` for `Memory` that `munmap`s the entire VA range and cleans up shared mappings

## 2. Shared Memory Registry Simplification

- [x] 2.1 Remove `ResolvedSharedMemoryMapping`, `SharedMemoryMappingState`, `LiveSharedMemoryMapping` from `shared_memory.rs`
- [x] 2.2 Remove all `read`/`write`/typed access methods from `ResolvedSharedMemoryMapping`
- [x] 2.3 Change `SharedRegion` backing from `Vec<u8>` to `*mut u8` (mmap'd)
- [x] 2.4 Implement `SharedRegion::new` using `mmap` with `MAP_SHARED | MAP_ANONYMOUS`
- [x] 2.5 Simplify `SharedMemoryRegistry` to: `next_region_id`, `regions: HashMap<SharedRegionId, Arc<SharedRegion>>`
- [x] 2.6 Update `allocate_region` to mmap the region and map it into the calling guest's Memory
- [x] 2.7 Implement `attach_region` that maps an existing region into the target guest's Memory
- [x] 2.8 Implement `free_region` that `munmap`s pages from guest Memory and decrements attachment count
- [x] 2.9 Implement per-page `mprotect` when `reader_slot` is specified on attach

## 3. Host Function Registration

- [x] 3.1 Define host function signatures for `alloc_region`, `free_region`, `attach_region` in `instance.rs`
- [x] 3.2 Register host functions as imports under the `"selium"` module namespace
- [x] 3.3 Implement `alloc_region` host function: allocates region, extends guest Memory, returns `(region_id, page_offset)`
- [x] 3.4 Implement `free_region` host function: unmaps pages, decrements count, frees if zero
- [x] 3.5 Implement `attach_region` host function: maps existing region into calling guest, returns `page_offset`
- [x] 3.6 Wire `Instance::drop` to auto-call `free_region` on all attached regions

## 4. Interpreter & Bounds Updates

- [x] 4.1 Verify `memory.atomic.wait32`/`notify` bounds checks use `Memory::len_bytes()` (covers shared pages)
- [x] 4.2 Verify `memory.load`/`store` bounds checks use extended memory size
- [x] 4.3 Verify `memory.copy`/`fill`/`init` bounds checks use extended memory size
- [x] 4.4 Add trap translation: `SIGSEGV` on shared region access → `TrapCode::MemoryOutOfBounds`

## 5. Snapshot Updates

- [x] 5.1 Update `MemorySnapshot` to include `shared_mappings: Vec<(u32, SharedRegionId)>`
- [x] 5.2 Implement snapshot serialization: copy owned pages, record shared page references by region ID
- [x] 5.3 Implement snapshot restore: re-attach shared regions by ID, restore owned pages
- [x] 5.4 Add error path: restore fails if a referenced region no longer exists

## 6. AOT Runtime Mirroring

- [x] 6.1 Mirror `Memory` mmap changes in `src/aot_runtime/runtime.rs`
- [x] 6.2 Mirror `SharedMemoryRegistry` simplification in AOT path
- [x] 6.3 Register same three host functions for AOT instances
- [x] 6.4 Update `ensure_jit_inactive_for_external_mutation` guards for mmap-based attach/detach
- [x] 6.5 Handle JIT code pointer invalidation when shared pages are unmapped

## 7. Public API

- [x] 7.1 Expose `alloc_region`, `free_region`, `attach_region` on `WasmApplication` for programmatic use
- [x] 7.2 Add `RegionProt` enum to public API
- [x] 7.3 Update examples and documentation to use new shared memory API

## 8. Tests

- [x] 8.1 Add unit test: Memory created with mmap, grow extends accessible range
- [x] 8.2 Add unit test: shared region mapped into guest memory, writes visible via direct pointer access
- [x] 8.3 Add unit test: `free_region` unmaps pages, subsequent access traps
- [x] 8.4 Add unit test: consumer with reader slot can write its cursor page but not data pages
- [x] 8.5 Add unit test: snapshot skips shared pages, restore re-attaches by ID
- [x] 8.6 Add unit test: `memory.atomic.wait32`/`notify` work across shared pages between two instances
- [x] 8.7 Add unit test: exponential backoff resolves contention between concurrent writers
- [x] 8.8 Update existing shared memory tests to use new mmap-based API
