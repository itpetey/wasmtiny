use crate::runtime::{Result, TrapCode, WasmError};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

fn shared_memory_alignment_error(kind: &str, alignment: u32, offset: u32) -> WasmError {
    WasmError::Runtime(format!(
        "shared memory {kind} offset {} must be aligned to {} bytes",
        offset, alignment
    ))
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Shared memory mapping id.
pub struct SharedMemoryMappingId(u64);

impl SharedMemoryMappingId {
    /// Constant `fn`.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Constant `fn`.
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shared memory mapping.
pub struct SharedMemoryMapping {
    mapping_id: SharedMemoryMappingId,
    region_id: SharedRegionId,
    region_offset: u32,
    len: u32,
    alignment: u32,
}

impl SharedMemoryMapping {
    /// Constant `fn`.
    pub const fn mapping_id(self) -> SharedMemoryMappingId {
        self.mapping_id
    }

    /// Constant `fn`.
    pub const fn region_id(self) -> SharedRegionId {
        self.region_id
    }

    /// Constant `fn`.
    pub const fn region_offset(self) -> u32 {
        self.region_offset
    }

    /// Constant `fn`.
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Constant `fn`.
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Constant `fn`.
    pub const fn alignment(self) -> u32 {
        self.alignment
    }

    /// Returns whether this mapping overlaps the given region range.
    pub fn overlaps_region_range(
        &self,
        region_id: SharedRegionId,
        region_offset: u32,
        len: u32,
    ) -> bool {
        if self.region_id != region_id {
            return false;
        }

        let self_start = self.region_offset as u64;
        let self_end = self_start + self.len as u64;
        let other_start = region_offset as u64;
        let other_end = other_start + len as u64;
        self_start < other_end && other_start < self_end
    }

    fn access_range(&self, offset: u32, access_len: usize) -> Result<std::ops::Range<usize>> {
        let access_len =
            u32::try_from(access_len).map_err(|_| WasmError::Trap(TrapCode::MemoryOutOfBounds))?;
        let end = offset
            .checked_add(access_len)
            .ok_or(WasmError::Trap(TrapCode::MemoryOutOfBounds))?;
        if end > self.len {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }

        let start = self
            .region_offset
            .checked_add(offset)
            .ok_or(WasmError::Trap(TrapCode::MemoryOutOfBounds))?;
        let end = start
            .checked_add(access_len)
            .ok_or(WasmError::Trap(TrapCode::MemoryOutOfBounds))?;
        Ok(start as usize..end as usize)
    }
}

#[derive(Debug)]
struct SharedRegion {
    alignment: u32,
    data: RwLock<Vec<u8>>,
    base_offset: usize,
    len: usize,
    attachment_count: AtomicUsize,
}

impl SharedRegion {
    fn new(size: u32, alignment: u32) -> Result<Self> {
        let backing_len = (size as usize)
            .checked_add(alignment as usize - 1)
            .ok_or_else(|| WasmError::Runtime("shared region allocation failed".to_string()))?;
        let mut data = Vec::new();
        data.try_reserve_exact(backing_len)
            .map_err(|_| WasmError::Runtime("shared region allocation failed".to_string()))?;
        data.resize(backing_len, 0);
        let base_offset = data.as_ptr().align_offset(alignment as usize);
        if base_offset == usize::MAX || base_offset + size as usize > data.len() {
            return Err(WasmError::Runtime(
                "shared region allocation failed".to_string(),
            ));
        }

        Ok(Self {
            alignment,
            data: RwLock::new(data),
            base_offset,
            len: size as usize,
            attachment_count: AtomicUsize::new(0),
        })
    }

    fn len(&self) -> usize {
        self.len
    }

    fn storage_range(&self, range: std::ops::Range<usize>) -> Result<std::ops::Range<usize>> {
        let start = self
            .base_offset
            .checked_add(range.start)
            .ok_or_else(|| WasmError::Runtime("shared region offset overflowed".to_string()))?;
        let end = self
            .base_offset
            .checked_add(range.end)
            .ok_or_else(|| WasmError::Runtime("shared region offset overflowed".to_string()))?;
        Ok(start..end)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedSharedMemoryMapping {
    mapping: SharedMemoryMapping,
    region: Arc<SharedRegion>,
    state: Arc<SharedMemoryMappingState>,
}

#[derive(Debug)]
struct SharedMemoryMappingState {
    active: AtomicBool,
    gate: RwLock<()>,
}

impl SharedMemoryMappingState {
    fn new() -> Self {
        Self {
            active: AtomicBool::new(true),
            gate: RwLock::new(()),
        }
    }
}

#[derive(Debug, Clone)]
struct LiveSharedMemoryMapping {
    mapping: SharedMemoryMapping,
    state: Arc<SharedMemoryMappingState>,
}

impl ResolvedSharedMemoryMapping {
    pub(crate) fn read(&self, offset: u32, buf: &mut [u8]) -> Result<()> {
        let _gate = self.state.gate.read();
        if !self.state.active.load(Ordering::SeqCst) {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is detached or not attached",
                self.mapping.mapping_id().raw()
            )));
        }
        let range = self.mapping.access_range(offset, buf.len())?;
        let range = self.region.storage_range(range)?;
        let data = self.region.data.read();
        buf.copy_from_slice(&data[range]);
        Ok(())
    }

    pub(crate) fn write(&self, offset: u32, buf: &[u8]) -> Result<()> {
        let _gate = self.state.gate.read();
        if !self.state.active.load(Ordering::SeqCst) {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is detached or not attached",
                self.mapping.mapping_id().raw()
            )));
        }
        let range = self.mapping.access_range(offset, buf.len())?;
        let range = self.region.storage_range(range)?;
        let mut data = self.region.data.write();
        data[range].copy_from_slice(buf);
        Ok(())
    }

    pub(crate) fn read_u8(&self, offset: u32) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.read(offset, &mut buf)?;
        Ok(buf[0])
    }

    pub(crate) fn write_u8(&self, offset: u32, value: u8) -> Result<()> {
        self.write(offset, &[value])
    }

    pub(crate) fn read_i32(&self, offset: u32) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.read(offset, &mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    pub(crate) fn write_i32(&self, offset: u32, value: i32) -> Result<()> {
        self.write(offset, &value.to_le_bytes())
    }

    pub(crate) fn read_i64(&self, offset: u32) -> Result<i64> {
        let mut buf = [0u8; 8];
        self.read(offset, &mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    pub(crate) fn write_i64(&self, offset: u32, value: i64) -> Result<()> {
        self.write(offset, &value.to_le_bytes())
    }

    pub(crate) fn read_f32(&self, offset: u32) -> Result<f32> {
        let mut buf = [0u8; 4];
        self.read(offset, &mut buf)?;
        Ok(f32::from_bits(u32::from_le_bytes(buf)))
    }

    pub(crate) fn write_f32(&self, offset: u32, value: f32) -> Result<()> {
        self.write(offset, &value.to_bits().to_le_bytes())
    }

    pub(crate) fn read_f64(&self, offset: u32) -> Result<f64> {
        let mut buf = [0u8; 8];
        self.read(offset, &mut buf)?;
        Ok(f64::from_bits(u64::from_le_bytes(buf)))
    }

    pub(crate) fn write_f64(&self, offset: u32, value: f64) -> Result<()> {
        self.write(offset, &value.to_bits().to_le_bytes())
    }
}

#[derive(Debug)]
/// Shared memory registry.
pub struct SharedMemoryRegistry {
    next_region_id: u64,
    next_mapping_id: u64,
    regions: HashMap<SharedRegionId, Arc<SharedRegion>>,
    live_mappings: HashMap<SharedMemoryMappingId, LiveSharedMemoryMapping>,
}

impl Default for SharedMemoryRegistry {
    fn default() -> Self {
        Self {
            next_region_id: 1,
            next_mapping_id: 1,
            regions: HashMap::new(),
            live_mappings: HashMap::new(),
        }
    }
}

impl SharedMemoryRegistry {
    /// Allocates region.
    pub fn allocate_region(&mut self, size: u32, alignment: u32) -> Result<SharedRegionId> {
        if size == 0 {
            return Err(WasmError::Runtime(
                "shared region size must be greater than zero".to_string(),
            ));
        }
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(WasmError::Runtime(
                "shared region alignment must be a non-zero power of two".to_string(),
            ));
        }

        let region_id = SharedRegionId(self.next_region_id);
        self.next_region_id += 1;
        self.regions
            .insert(region_id, Arc::new(SharedRegion::new(size, alignment)?));
        Ok(region_id)
    }

    /// Returns the length of the shared region in bytes.
    pub fn region_len(&self, region_id: SharedRegionId) -> Result<u32> {
        let region = self.region(region_id)?;
        Ok(region.len() as u32)
    }

    /// Destroys region.
    pub fn destroy_region(&mut self, region_id: SharedRegionId) -> Result<()> {
        let region = self.region(region_id)?;
        if region.attachment_count.load(Ordering::SeqCst) != 0 {
            return Err(WasmError::Runtime(format!(
                "shared region {} still has attached mappings",
                region_id.raw()
            )));
        }

        self.regions.remove(&region_id);
        Ok(())
    }

    /// Attaches region.
    pub fn attach_region(
        &mut self,
        region_id: SharedRegionId,
        region_offset: u32,
        len: u32,
    ) -> Result<SharedMemoryMapping> {
        if len == 0 {
            return Err(WasmError::Runtime(
                "shared memory attachment length must be greater than zero".to_string(),
            ));
        }

        let region = self.region(region_id)?;
        if !region_offset.is_multiple_of(region.alignment) {
            return Err(shared_memory_alignment_error(
                "attachment",
                region.alignment,
                region_offset,
            ));
        }
        let mapping = SharedMemoryMapping {
            mapping_id: SharedMemoryMappingId(self.next_mapping_id),
            region_id,
            region_offset,
            len,
            alignment: region.alignment,
        };
        self.next_mapping_id += 1;

        let range = mapping.access_range(0, len as usize)?;
        if range.end > region.len() {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }

        region.attachment_count.fetch_add(1, Ordering::SeqCst);
        self.live_mappings.insert(
            mapping.mapping_id(),
            LiveSharedMemoryMapping {
                mapping,
                state: Arc::new(SharedMemoryMappingState::new()),
            },
        );
        Ok(mapping)
    }

    /// Detaches region.
    pub fn detach_region(&mut self, mapping: SharedMemoryMapping) -> Result<()> {
        let stored_mapping = self
            .live_mappings
            .get(&mapping.mapping_id())
            .cloned()
            .ok_or_else(|| {
                WasmError::Runtime(format!(
                    "shared memory mapping {} is already detached",
                    mapping.mapping_id().raw()
                ))
            })?;
        if stored_mapping.mapping != mapping {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is invalid",
                mapping.mapping_id().raw()
            )));
        }

        let _gate = stored_mapping.state.gate.write();
        if !stored_mapping.state.active.swap(false, Ordering::SeqCst) {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is already detached",
                mapping.mapping_id().raw()
            )));
        }

        let region = self.region(mapping.region_id())?;
        let count = region.attachment_count.load(Ordering::SeqCst);
        if count == 0 {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is already detached",
                mapping.mapping_id().raw()
            )));
        }

        region.attachment_count.fetch_sub(1, Ordering::SeqCst);
        self.live_mappings.remove(&mapping.mapping_id());
        Ok(())
    }

    pub(crate) fn resolve_mapping(
        &self,
        mapping: SharedMemoryMapping,
    ) -> Result<ResolvedSharedMemoryMapping> {
        let live_mapping = self.validate_mapping(mapping)?;
        Ok(ResolvedSharedMemoryMapping {
            mapping,
            region: self.region(mapping.region_id())?,
            state: live_mapping.state.clone(),
        })
    }

    fn region(&self, region_id: SharedRegionId) -> Result<Arc<SharedRegion>> {
        self.regions.get(&region_id).cloned().ok_or_else(|| {
            WasmError::Runtime(format!("shared region {} not found", region_id.raw()))
        })
    }

    fn validate_mapping(&self, mapping: SharedMemoryMapping) -> Result<&LiveSharedMemoryMapping> {
        let Some(stored_mapping) = self.live_mappings.get(&mapping.mapping_id()) else {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is detached or not attached",
                mapping.mapping_id().raw()
            )));
        };
        if stored_mapping.mapping != mapping {
            return Err(WasmError::Runtime(format!(
                "shared memory mapping {} is invalid",
                mapping.mapping_id().raw()
            )));
        }

        Ok(stored_mapping)
    }
}
