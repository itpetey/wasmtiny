//! Memory management for WebAssembly instances.
//!
//! This module provides the [`Memory`] type which represents a WebAssembly linear
//! memory. It handles allocation, growth, reading, and writing of memory pages.
//!
//! WebAssembly memories are defined by their page count, where each page is 64 KiB.
//! The runtime supports both minimum and maximum page limits.
//!
//! # Memory Operations
//!
//! - Create: `Memory::new(memory_type)`
//! - Read: `memory.read(offset, buffer)`
//! - Write: `memory.write(offset, data)`
//! - Grow: `memory.grow(pages)`

use crate::runtime::{InstanceMeter, MemoryType, Result, TrapCode, WasmError};
use std::sync::{Arc, Weak};

/// Constant `PAGE_SIZE_BYTES`.
pub const PAGE_SIZE_BYTES: u32 = 65536;
const MAX_PAGES: u32 = 65536;

/// WebAssembly linear memory.
///
/// A linear memory is a contiguous array of bytes that can be read from and
/// written to by WebAssembly code. Memory grows in units of 64 KiB pages.
///
/// # Example
///
/// ```
/// use wasmtiny::runtime::{MemoryType, Limits, Memory};
///
/// let mem_type = MemoryType::new(Limits::Min(1));
/// let mut mem = Memory::new(mem_type);
/// assert_eq!(mem.size(), 1);
///
/// mem.grow(1).unwrap();
/// assert_eq!(mem.size(), 2);
/// ```
#[derive(Debug, Clone)]
/// Memory.
pub struct Memory {
    mem_type: MemoryType,
    data: Vec<u8>,
    meters: Vec<Weak<InstanceMeter>>,
}

impl Memory {
    /// Creates a new `Memory`.
    pub fn new(mem_type: MemoryType) -> Self {
        let min_pages = mem_type.limits.min();
        let byte_len = (min_pages * PAGE_SIZE_BYTES) as usize;
        Self {
            mem_type,
            data: vec![0; byte_len],
            meters: Vec::new(),
        }
    }

    /// Returns the size.
    pub fn size(&self) -> u32 {
        (self.data.len() / PAGE_SIZE_BYTES as usize) as u32
    }

    /// Returns the declared type information.
    pub fn type_(&self) -> &MemoryType {
        &self.mem_type
    }

    /// Grows the underlying resource by the requested delta.
    pub fn grow(&mut self, delta: u32) -> Result<u32> {
        let old_size = self.size();
        let new_size = old_size.saturating_add(delta);

        self.prune_meters();
        for meter in self.meters.iter().filter_map(Weak::upgrade) {
            meter.ensure_memory_pages(new_size)?;
        }

        if let Some(max) = self.mem_type.limits.max()
            && new_size > max
        {
            return Err(WasmError::Runtime(
                "memory size exceeds maximum".to_string(),
            ));
        }

        if new_size > MAX_PAGES {
            return Err(WasmError::Runtime(
                "memory size exceeds maximum allowed".to_string(),
            ));
        }

        let new_byte_len = (new_size * PAGE_SIZE_BYTES) as usize;
        self.data.resize(new_byte_len, 0);
        Ok(old_size)
    }

    /// Reads bytes from the underlying resource.
    pub fn read(&self, offset: u32, buf: &mut [u8]) -> Result<()> {
        let offset = offset as usize;
        if buf.is_empty() {
            if offset > self.data.len() {
                return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
            }
            return Ok(());
        }
        if offset >= self.data.len() {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }
        if offset + buf.len() > self.data.len() {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }
        buf.copy_from_slice(&self.data[offset..offset + buf.len()]);
        Ok(())
    }

    /// Writes bytes to the underlying resource.
    pub fn write(&mut self, offset: u32, buf: &[u8]) -> Result<()> {
        let offset = offset as usize;
        if buf.is_empty() {
            if offset > self.data.len() {
                return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
            }
            return Ok(());
        }
        if offset >= self.data.len() {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }
        if offset + buf.len() > self.data.len() {
            return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
        }
        self.data[offset..offset + buf.len()].copy_from_slice(buf);
        Ok(())
    }

    /// Reads u8.
    pub fn read_u8(&self, offset: u32) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.read(offset, &mut buf)?;
        Ok(buf[0])
    }

    /// Writes u8.
    pub fn write_u8(&mut self, offset: u32, val: u8) -> Result<()> {
        self.write(offset, &[val])
    }

    /// Reads u32.
    pub fn read_u32(&self, offset: u32) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read(offset, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Writes u32.
    pub fn write_u32(&mut self, offset: u32, val: u32) -> Result<()> {
        self.write(offset, &val.to_le_bytes())
    }

    /// Reads i32.
    pub fn read_i32(&self, offset: u32) -> Result<i32> {
        Ok(self.read_u32(offset)? as i32)
    }

    /// Writes i32.
    pub fn write_i32(&mut self, offset: u32, val: i32) -> Result<()> {
        self.write_u32(offset, val as u32)
    }

    /// Reads u64.
    pub fn read_u64(&self, offset: u32) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.read(offset, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    /// Writes u64.
    pub fn write_u64(&mut self, offset: u32, val: u64) -> Result<()> {
        self.write(offset, &val.to_le_bytes())
    }

    /// Reads i64.
    pub fn read_i64(&self, offset: u32) -> Result<i64> {
        Ok(self.read_u64(offset)? as i64)
    }

    /// Writes i64.
    pub fn write_i64(&mut self, offset: u32, val: i64) -> Result<()> {
        self.write_u64(offset, val as u64)
    }

    /// Reads f32.
    pub fn read_f32(&self, offset: u32) -> Result<f32> {
        Ok(f32::from_bits(self.read_u32(offset)?))
    }

    /// Writes f32.
    pub fn write_f32(&mut self, offset: u32, val: f32) -> Result<()> {
        self.write_u32(offset, val.to_bits())
    }

    /// Reads f64.
    pub fn read_f64(&self, offset: u32) -> Result<f64> {
        Ok(f64::from_bits(self.read_u64(offset)?))
    }

    /// Writes f64.
    pub fn write_f64(&mut self, offset: u32, val: f64) -> Result<()> {
        self.write_u64(offset, val.to_bits())
    }

    /// Returns a mutable pointer to the underlying buffer.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    /// Returns the length of the underlying buffer in bytes.
    pub fn len_bytes(&self) -> usize {
        self.data.len()
    }

    /// Returns the underlying data slice.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the underlying data slice mutably.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[..]
    }

    pub(crate) fn attach_meter(&mut self, meter: &Arc<InstanceMeter>) {
        self.prune_meters();
        if self
            .meters
            .iter()
            .filter_map(Weak::upgrade)
            .any(|existing| Arc::ptr_eq(&existing, meter))
        {
            return;
        }

        self.meters.push(Arc::downgrade(meter));
    }

    fn prune_meters(&mut self) {
        self.meters.retain(|meter| meter.strong_count() > 0);
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Limits;

    use super::*;

    #[test]
    fn test_memory_creation() {
        let mem_type = MemoryType::new(Limits::Min(1));
        let mem = Memory::new(mem_type);
        assert_eq!(mem.size(), 1);
    }

    #[test]
    fn test_memory_grow() {
        let mut mem = Memory::new(MemoryType::new(Limits::Min(1)));
        assert_eq!(mem.size(), 1);
        let old = mem.grow(1).unwrap();
        assert_eq!(old, 1);
        assert_eq!(mem.size(), 2);
    }

    #[test]
    fn test_memory_read_write() {
        let mut mem = Memory::new(MemoryType::new(Limits::Min(1)));
        mem.write_i32(0, 42).unwrap();
        assert_eq!(mem.read_i32(0).unwrap(), 42);
    }

    #[test]
    fn test_memory_out_of_bounds() {
        let mem = Memory::new(MemoryType::new(Limits::Min(1)));
        assert!(mem.read(65536, &mut [0]).is_err());
    }

    #[test]
    fn test_memory_zero_length_access_at_end_is_allowed() {
        let mut mem = Memory::new(MemoryType::new(Limits::Min(1)));
        let mut empty = [];

        mem.read(65536, &mut empty).unwrap();
        mem.write(65536, &[]).unwrap();
    }
}
