## Why

Current `Memory` is `Vec<u8>`-backed and shared regions are accessed through `ResolvedSharedMemoryMapping::read/write` — a byte-copy path that prevents guests from using native WASM load/store and atomic instructions on shared data. To enable the "dumb host, smart guest" messaging model where guests build ring buffers and synchronization on top of mapped memory pages, the runtime must support mapping external shared regions directly into guest linear memory.

## What Changes

- **BREAKING**: Switch guest linear memory backing from `Vec<u8>` to `mmap`-based allocation — `Memory.grow()` uses `mremap` (Linux) instead of `Vec::resize`
- Add shared region mapping: `alloc_region` mmaps a new region and extends guest memory address space to include it; `attach_region` maps an existing region into a different instance; `free_region` unmaps and releases
- **BREAKING**: Remove `ResolvedSharedMemoryMapping` and `SharedMemoryMappingState` — the byte-level read/write path with the `active` flag and `RwLock` gate is replaced by direct memory access through the guest's extended address space; detach becomes `munmap` from guest memory
- Track which pages are owned vs. shared in `Memory` for correct cleanup, snapshot exclusion, and bounds checking
- Ensure `memory.atomic.wait32`/`notify` cover the full extended address range including shared pages
- Update snapshot system to handle split owned/shared page ranges: owned pages are copied, shared pages are skipped or referenced by region ID
- **BREAKING**: Replace `SharedMemoryMapping`/`SharedMemoryMappingId` with `RegionProt`-aware mapping metadata on the `Memory` struct itself — mappings are tracked as page ranges rather than separate handle objects
- Mirror all changes in the AOT runtime path

## Capabilities

### New Capabilities
- `mmap-backed-memory`: Guest linear memory backed by `mmap` instead of `Vec<u8>`, enabling `mremap`-based growth and external page mapping
- `shared-region-mapping`: Ability to map external shared memory regions into guest linear memory address space so guests access them via native load/store/atomic instructions

### Modified Capabilities
- `shared-memory-regions`: Remove `ResolvedSharedMemoryMapping` byte-copy read/write path; replace with mmap-based mapping into guest linear memory; remove `SharedMemoryMappingState`; simplify registry to lifecycle tracking only
- `wasm-runtime-core`: `Memory` type changes from `Vec<u8>` to mmap-backed allocation; `grow()` uses `mremap`; `read()`/`write()` operate on raw pointers; new shared page tracking

## Impact

| Area | Impact |
|------|--------|
| `src/memory.rs` | Core data structure rewrite — `Vec<u8>` replaced with `*mut u8` + `len` + `capacity` + `shared_ranges: Vec<SharedRange>` |
| `src/runtime/shared_memory.rs` | Remove `ResolvedSharedMemoryMapping`, `SharedMemoryMappingState`, all byte read/write methods; add mmap-based region mapping into guest `Memory` |
| `src/runtime/instance.rs` | Three new host functions registered as imports; memory bounds account for shared pages; `Instance::drop` unmaps shared regions |
| `src/interpreter/exec.rs` | Verify memory bounds for `memory.atomic.wait32`/`notify` cover full extended range |
| `src/runtime/snapshot.rs` | Handle mmap-backed memory and shared page exclusion from snapshots |
| `src/aot_runtime/runtime.rs` | Mirror all Memory and shared region changes for AOT path |
| `src/application.rs` | Public API exposes the three region host functions |
