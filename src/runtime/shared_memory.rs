use crate::memory::{PAGE_SIZE_BYTES, RegionProt};
use crate::runtime::{Memory, Result, WasmError};
use parking_lot::{Condvar, Mutex as ParkingMutex, RwLock};
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

/// A waiter for atomic wait/notify on shared memory addresses.
#[derive(Debug)]
pub(crate) struct SharedWaiter {
    pub(crate) notified: ParkingMutex<bool>,
    pub(crate) condvar: Condvar,
}

/// Monotonic counter for generating unique shm names.
static NEXT_SHM_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Shared region id.
pub struct SharedRegionId(u64);

impl SharedRegionId {
    /// Constant `fn`.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Constant `fn`.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// An mmap-backed shared memory region.
///
/// Shared regions are backed by `shm_open` to obtain a file descriptor, then
/// mapped with `mmap(MAP_SHARED)`. The same fd is used with `mmap(MAP_FIXED |
/// MAP_SHARED)` to map the identical physical pages into multiple guest address
/// spaces, achieving true cross-instance visibility without software copies.
pub struct SharedRegion {
    /// Base pointer of the creator's mmap of the region.
    ptr: *mut u8,
    /// Length in bytes (page-aligned).
    len: usize,
    /// File descriptor from shm_open; kept alive so guests can MAP_FIXED the
    /// same pages. Closed on Drop.
    fd: i32,
    /// Number of guest instances that currently have this region mapped.
    attachment_count: AtomicUsize,
    /// Shared waiters for atomic wait/notify on addresses within this region.
    /// Keyed by byte offset within the region (not guest address).
    waiters: Arc<RwLock<HashMap<u32, Arc<SharedWaiter>>>>,
}

// SAFETY: The fd and ptr are process-wide resources. The fd is a plain integer
// and the ptr points to a shared mapping that the kernel serialises. All
// mutation of attachment_count is atomic.
unsafe impl Send for SharedRegion {}
unsafe impl Sync for SharedRegion {}

impl std::fmt::Debug for SharedRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedRegion")
            .field("len", &self.len)
            .field("fd", &self.fd)
            .field(
                "attachment_count",
                &self.attachment_count.load(Ordering::SeqCst),
            )
            .finish()
    }
}

impl SharedRegion {
    /// Creates a new shared region backed by `shm_open` + `mmap(MAP_SHARED)`.
    ///
    /// The shared memory object is unlinked immediately after creation so it
    /// does not persist in the filesystem namespace; the fd keeps it alive
    /// until all mappings are released and the fd is closed.
    fn new(size: u32) -> Result<Self> {
        if size == 0 {
            return Err(WasmError::Runtime(
                "shared region size must be greater than zero".to_string(),
            ));
        }

        let len = size as usize;

        // Generate a unique name for the POSIX shared memory object.
        let id = NEXT_SHM_ID.fetch_add(1, Ordering::Relaxed);
        let name = format!("/wasmtiny_shm_{}", id);
        let c_name = CString::new(name.as_bytes())
            .map_err(|_| WasmError::Runtime("failed to create shm name".to_string()))?;

        // Create the shared memory object.
        // SAFETY: c_name is a valid C string. O_CREAT | O_RDWR with mode 0600.
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o600,
            )
        };
        if fd < 0 {
            return Err(WasmError::Runtime(format!(
                "shm_open failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // Unlink immediately — the fd keeps the object alive.
        // SAFETY: c_name is still valid.
        unsafe {
            libc::shm_unlink(c_name.as_ptr());
        }

        // Set the size of the shared memory object.
        // SAFETY: fd is a valid open file descriptor.
        if unsafe { libc::ftruncate(fd, len as libc::off_t) } != 0 {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(WasmError::Runtime(format!(
                "ftruncate failed for shared region: {}",
                err
            )));
        }

        // Map the shared memory object into the creator's address space.
        // SAFETY: fd is valid and the object is at least `len` bytes.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(WasmError::Runtime(format!(
                "mmap failed for shared region: {}",
                err
            )));
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            len,
            fd,
            attachment_count: AtomicUsize::new(0),
            waiters: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Returns the base pointer of the creator's mapping.
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Returns the file descriptor backing this shared region.
    ///
    /// Used by `Memory::map_shared_region` to `mmap(MAP_FIXED | MAP_SHARED)`
    /// the same physical pages into a guest's address space.
    pub fn fd(&self) -> i32 {
        self.fd
    }

    /// Returns a reference to the shared waiters Arc.
    ///
    /// Used by `Memory::map_shared_region` to share waiters across instances.
    pub(crate) fn waiters_arc(&self) -> Arc<RwLock<HashMap<u32, Arc<SharedWaiter>>>> {
        self.waiters.clone()
    }

    /// Returns the length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the region is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the current attachment count.
    pub fn attachment_count(&self) -> usize {
        self.attachment_count.load(Ordering::SeqCst)
    }

    /// Increments the attachment count.
    fn attach(&self) {
        self.attachment_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the attachment count.
    fn detach(&self) {
        self.attachment_count.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Drop for SharedRegion {
    fn drop(&mut self) {
        // Unmap the creator's mapping first.
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: ptr was allocated via mmap with self.len bytes.
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.len);
            }
        }
        // Close the fd. The kernel frees the shared memory object once all
        // mappings (including those in guest address spaces) are released.
        if self.fd >= 0 {
            // SAFETY: fd was obtained from shm_open and has not been closed.
            unsafe {
                libc::close(self.fd);
            }
        }
    }
}

/// Shared memory registry.
///
/// Manages the lifecycle of shared memory regions: creation, destruction,
/// attachment to guest instances, and detachment. Regions are mapped directly
/// into guest linear memory via `mmap(MAP_FIXED | MAP_SHARED)` using a shared
/// file descriptor, so writes in one guest are visible to all others without
/// any software copy path.
///
/// The registry provides **no** public byte-level read or write methods; all
/// data access goes through the guest's native load/store instructions on the
/// mapped pages. Host-side convenience accessors are `pub(crate)` only.
pub struct SharedMemoryRegistry {
    next_region_id: u64,
    regions: HashMap<SharedRegionId, Arc<SharedRegion>>,
}

impl std::fmt::Debug for SharedMemoryRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedMemoryRegistry")
            .field("next_region_id", &self.next_region_id)
            .field("regions", &self.regions.len())
            .finish()
    }
}

impl Default for SharedMemoryRegistry {
    fn default() -> Self {
        Self {
            next_region_id: 1,
            regions: HashMap::new(),
        }
    }
}

impl SharedMemoryRegistry {
    /// Allocates a new shared region and maps it into the calling guest's memory.
    ///
    /// Returns `(region_id, page_offset)` where `page_offset` is the guest page
    /// where the region was mapped.
    pub fn allocate_region(
        &mut self,
        memory: &mut Memory,
        size: u32,
        prot: RegionProt,
    ) -> Result<(SharedRegionId, u32)> {
        if size == 0 {
            return Err(WasmError::Runtime(
                "shared region size must be greater than zero".to_string(),
            ));
        }

        // Round up to page boundary
        let page_size = PAGE_SIZE_BYTES;
        let aligned_size = size.div_ceil(page_size) * page_size;

        let region = SharedRegion::new(aligned_size)?;
        let region_id = SharedRegionId(self.next_region_id);
        self.next_region_id += 1;

        // Map into guest memory using the shared fd (true shared pages).
        let page_offset = memory.map_shared_region(
            region.fd,
            region.len,
            region_id,
            prot,
            None,
            region.waiters_arc(),
        )?;

        region.attach();
        self.regions.insert(region_id, Arc::new(region));

        Ok((region_id, page_offset))
    }

    /// Allocates a region without mapping it into any guest memory.
    /// Used for creating regions that will be attached later.
    pub fn allocate_region_standalone(&mut self, size: u32) -> Result<SharedRegionId> {
        if size == 0 {
            return Err(WasmError::Runtime(
                "shared region size must be greater than zero".to_string(),
            ));
        }

        let page_size = PAGE_SIZE_BYTES;
        let aligned_size = size.div_ceil(page_size) * page_size;

        let region = SharedRegion::new(aligned_size)?;
        let region_id = SharedRegionId(self.next_region_id);
        self.next_region_id += 1;

        self.regions.insert(region_id, Arc::new(region));
        Ok(region_id)
    }

    /// Returns the length of the shared region in bytes.
    pub fn region_len(&self, region_id: SharedRegionId) -> Result<u32> {
        let region = self.region(region_id)?;
        Ok(region.len() as u32)
    }

    /// Returns a reference to the shared region (crate-internal).
    pub(crate) fn get_region(&self, region_id: SharedRegionId) -> Result<Arc<SharedRegion>> {
        self.region(region_id)
    }

    /// Destroys a shared region.
    ///
    /// The region must have no attachments.
    pub fn destroy_region(&mut self, region_id: SharedRegionId) -> Result<()> {
        let region = self.region(region_id)?;
        if region.attachment_count() != 0 {
            return Err(WasmError::Runtime(format!(
                "shared region {} still has {} attached mappings",
                region_id.raw(),
                region.attachment_count()
            )));
        }

        self.regions.remove(&region_id);
        Ok(())
    }

    /// Attaches an existing shared region into a guest's memory.
    ///
    /// The region's physical pages are mapped into the guest's address space
    /// using `mmap(MAP_FIXED | MAP_SHARED)` with the region's fd, so writes
    /// are immediately visible to all other attached instances.
    ///
    /// Returns the page offset where the region was mapped.
    pub fn attach_region(
        &mut self,
        memory: &mut Memory,
        region_id: SharedRegionId,
        prot: RegionProt,
        reader_slot: Option<u32>,
    ) -> Result<u32> {
        let region = self.region(region_id)?;

        let page_offset = memory.map_shared_region(
            region.fd,
            region.len,
            region_id,
            prot,
            reader_slot,
            region.waiters_arc(),
        )?;

        region.attach();
        Ok(page_offset)
    }

    /// Detaches a shared region from a guest's memory.
    ///
    /// Unmaps the region's pages from the guest's address space and restores
    /// the virtual address reservation to `PROT_NONE`.
    pub fn detach_region(&mut self, memory: &mut Memory, region_id: SharedRegionId) -> Result<()> {
        let region = self.region(region_id)?;

        memory.unmap_shared_region(region_id)?;
        region.detach();

        Ok(())
    }

    /// Writes data to a shared region directly (host-side convenience).
    ///
    /// This is `pub(crate)` — the registry's public API has no read/write
    /// methods per the shared-region-mapping spec. Host callers should go
    /// through `Instance` or `Store` wrappers instead.
    pub(crate) fn write_to_region(
        &self,
        region_id: SharedRegionId,
        offset: usize,
        data: &[u8],
    ) -> Result<()> {
        let region = self.region(region_id)?;
        if offset + data.len() > region.len() {
            return Err(WasmError::Runtime(
                "shared region write out of bounds".to_string(),
            ));
        }
        // SAFETY: offset and length are bounds-checked above; ptr is valid.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), region.ptr.add(offset), data.len());
        }
        Ok(())
    }

    /// Reads data from a shared region directly (host-side convenience).
    ///
    /// This is `pub(crate)` — see [`write_to_region`] for rationale.
    pub(crate) fn read_from_region(
        &self,
        region_id: SharedRegionId,
        offset: usize,
        buf: &mut [u8],
    ) -> Result<()> {
        let region = self.region(region_id)?;
        if offset + buf.len() > region.len() {
            return Err(WasmError::Runtime(
                "shared region read out of bounds".to_string(),
            ));
        }
        // SAFETY: offset and length are bounds-checked above; ptr is valid.
        unsafe {
            std::ptr::copy_nonoverlapping(region.ptr.add(offset), buf.as_mut_ptr(), buf.len());
        }
        Ok(())
    }

    fn region(&self, region_id: SharedRegionId) -> Result<Arc<SharedRegion>> {
        self.regions.get(&region_id).cloned().ok_or_else(|| {
            WasmError::Runtime(format!("shared region {} not found", region_id.raw()))
        })
    }
}
