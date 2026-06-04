## Context

`wasmtiny` currently uses `Vec<u8>` for guest linear memory (`Memory.data`) and a separate `SharedRegion` with `Vec<u8>` backing for cross-instance shared regions. Data flows between them through `ResolvedSharedMemoryMapping::read/write` — byte-level copy operations that require hostcalls and prevent guests from using native WASM load/store/atomic instructions on shared data.

The "dumb host, smart guest" model requires shared regions to appear in the guest's linear memory address space so guests can use `memory.atomic.wait32`/`notify` for synchronization and regular load/store for data access. This demands mmap-backed memory and the ability to map external pages into the guest's virtual address range.

## Goals / Non-Goals

**Goals:**
- Replace `Vec<u8>` with mmap-backed memory for guest linear memory
- Enable `grow()` via `mremap` (Linux) / platform-specific virtual address extension
- Map shared regions into guest linear memory so they're accessed via native WASM instructions
- Remove the `ResolvedSharedMemoryMapping` byte-copy read/write path
- Track owned vs. shared page ranges for snapshot exclusion and cleanup
- Mirror all changes in the AOT runtime

**Non-Goals:**
- Platform portability beyond Linux — macOS and Windows can follow as separate changes
- `MAP_HUGETLB` or transparent huge pages — standard 4KB pages only for initial implementation
- NUMA-aware allocation — regions are allocated on the local NUMA node
- Cross-process mmap sharing via file descriptors — all regions use `MAP_SHARED | MAP_ANONYMOUS`
- Changing the guard page or memory trap mechanisms

## Decisions

### 1. Memory backing: `mmap` with reserved address space

The `Memory` struct changes from:

```rust
struct Memory {
    data: Vec<u8>,          // owned linear memory
    ...
}
```

to:

```rust
struct Memory {
    ptr: *mut u8,           // base of mmap'd region
    len: usize,             // current valid length (bytes)
    capacity: usize,        // total reserved VA range (bytes)
    shared_ranges: Vec<SharedRange>,  // (page_offset, region_id, len)
    waiters: Arc<RwLock<HashMap<u32, Arc<Waiter>>>>,  // unchanged
    ...
}

struct SharedRange {
    page_offset: u32,       // offset in guest pages
    region_id: SharedRegionId,
    len: u32,               // length in bytes
    prot: RegionProt,
    reader_slot: Option<u32>,  // which page is writable by this consumer
}
```

**Initial allocation:** On `Memory::new(min_pages)`, `mmap` reserves `MAX_PAGES * PAGE_SIZE` (4GB) of virtual address space with `PROT_NONE`, then `mprotect`s the first `min_pages * PAGE_SIZE` to `PROT_READ | PROT_WRITE`. This avoids the need to move the allocation on grow.

**Growth:** `grow(delta)` calls `mprotect` to make the next `delta * PAGE_SIZE` bytes accessible. No `mremap` needed because the full VA range is pre-reserved.

**Rationale:** Pre-reserving the VA range avoids the complexity of `mremap` (which can move the allocation, invalidating all existing pointers). 4GB of virtual address space is free on 64-bit systems. The downside is that `MAX_PAGES` must be set at creation time, but this is already a WASM constraint (max 65536 pages = 4GB).

**Alternatives considered:**
- Keep `Vec<u8>` and use `mremap` for each grow — `mremap` can move the allocation, requiring pointer invalidation throughout the runtime
- Use `mmap` with `MREMAP_MAYMOVE` on each grow — same pointer invalidation problem
- Use a `Vec` of page-sized chunks (sparse memory) — higher overhead for bounds checks, complicates JIT codegen

### 2. Shared region mapping into guest memory

When `alloc_region` is called:

1. Host `mmap`s the region with `MAP_SHARED | MAP_ANONYMOUS`, size = `pages * PAGE_SIZE`
2. Host finds a free range in the guest's reserved VA space above the current memory size
3. Host `mmap`s the shared pages at that offset using `MAP_FIXED | MAP_SHARED` with the shared region fd (or equivalent)
4. Host records the mapping in `Memory.shared_ranges`
5. Returns `(region_id, page_offset)` to the guest

**Detach:** `free_region` calls `munmap` on the guest's mapping of those pages, restores `PROT_NONE`, and decrements the region's attachment count. The kernel serializes this with any in-flight loads/stores — no `RwLock` gate needed.

**Per-page protection:** When `attach_region` specifies `reader_slot: Some(n)`, the host maps everything `PROT_READ` first, then `mprotect`s the specific reader cursor page to `PROT_READ | PROT_WRITE`.

### 3. Pointer-based read/write instead of slice indexing

The current `Memory::read` and `Memory::write` do `self.data[offset..offset+len].copy_from_slice(buf)`. With shared pages not living in the same allocation, this doesn't work. The new approach:

```rust
impl Memory {
    pub fn read(&self, offset: u32, buf: &mut [u8]) -> Result<()> {
        let ptr = self.ptr_at(offset, buf.len())?;
        unsafe { std::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), buf.len()); }
        Ok(())
    }

    pub fn write(&mut self, offset: u32, buf: &[u8]) -> Result<()> {
        let ptr = self.ptr_at_mut(offset, buf.len())?;
        // Check if any byte falls in a read-only shared range
        self.check_writable(offset, buf.len())?;
        unsafe { std::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, buf.len()); }
        Ok(())
    }

    fn ptr_at(&self, offset: u32, len: usize) -> Result<*const u8> {
        // Bounds check against total memory size (owned + shared)
        // Return self.ptr.add(offset as usize)
    }

    fn check_writable(&self, offset: u32, len: usize) -> Result<()> {
        // Verify no byte in [offset, offset+len) falls in a PROT_READ shared range
        // This catches writes before they SIGSEGV, providing a WASM trap instead
    }
}
```

**Rationale:** The `check_writable` check before writes is a defense-in-depth measure. Even though the kernel will `SIGSEGV` a write to `PROT_READ` pages, catching it in software gives a clean WASM trap (`MemoryOutOfBounds` or a new `MemoryNotWritable`) rather than a Unix signal that the runtime must translate.

### 4. Removing `ResolvedSharedMemoryMapping`

The current path for every shared memory access:

```
Guest calls hostcall → host resolves mapping → checks active flag → takes read gate → copies bytes → returns
```

With memory mapping, this becomes:

```
Guest executes i32.load at offset → CPU resolves address → loads from memory
```

No hostcalls. No locks. No copy. The `SharedMemoryRegistry` is simplified to:

```rust
struct SharedMemoryRegistry {
    next_region_id: u64,
    regions: HashMap<SharedRegionId, Arc<SharedRegion>>,
}

struct SharedRegion {
    ptr: *mut u8,           // mmap'd region
    len: usize,
    attachment_count: AtomicUsize,
}
```

Lifecycle only. No read/write path. No mapping state. No gate.

### 5. Snapshot handling

Snapshots must handle two kinds of pages:

- **Owned pages** (below the `memory.size()` boundary and not in `shared_ranges`): copied into the snapshot as before
- **Shared pages** (in `shared_ranges`): skipped — they belong to the region, not the instance. The snapshot records `(page_offset, region_id)` references so restore can re-attach

```rust
struct MemorySnapshot {
    owned_pages: Vec<u8>,           // copy of owned memory
    owned_size: u32,                // pages
    shared_mappings: Vec<(u32, SharedRegionId)>,  // (page_offset, region_id)
}
```

On restore, the runtime re-attaches each shared region at the same page offset. If a region no longer exists, restore fails.

### 6. AOT runtime mirroring

The AOT runtime (`src/aot_runtime/runtime.rs`) has its own `Memory` management that parallels the interpreter path. The same changes apply:

- `Memory` switches to mmap-backed storage
- `SharedMemoryRegistry` loses the read/write path
- The `ensure_jit_inactive_for_external_mutation()` calls before attach/detach remain — they prevent JIT'd code from accessing pages being remapped

The AOT path also needs to handle the fact that JIT'd code holds raw pointers into memory. When shared regions are mapped or unmapped, cached code addresses may need invalidation. The existing safepoint mechanism handles this.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Pre-reserving 4GB VA per instance exhausts address space on 32-bit or many-instance deployments | Make `MAX_PAGES` configurable per `MemoryType`; default to 65536 on 64-bit, 1024 on 32-bit |
| `MAP_FIXED` clobbers existing mappings if the reserved range is wrong | Validate the target address range is `PROT_NONE` before mapping; abort with clear error if not |
| `mprotect` on reader cursor pages is a syscall per attach | Amortized over channel lifetime; one call per consumer per channel, not per operation |
| `check_writable` adds bounds-check overhead to every write | The check is a linear scan of `shared_ranges` — O(n) in number of shared ranges. In practice, n ≤ 4 (one data region + maybe RPC request/reply rings). Cache the "last matched range" for hot-path optimization |
| JIT'd code holds stale pointers after `munmap` of shared pages | Use the existing safepoint mechanism to pause JIT threads during attach/detach; flush any cached code addresses for the unmapped range |
| Snapshot of a running instance with shared pages may capture inconsistent state | Snapshots already require the instance to be paused (safepoint). Shared pages are skipped entirely — the snapshot references them by ID, and consistency is the region owner's responsibility |
