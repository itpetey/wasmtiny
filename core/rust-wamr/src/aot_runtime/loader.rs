use crate::runtime::{Result, WasmError};
use std::io::Cursor;

pub struct AotLoader;

impl AotLoader {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 4 {
            return Err(WasmError::Load("AOT data too short".to_string()));
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != 0x6D736100 {
            return Err(WasmError::Load("Invalid AOT magic".to_string()));
        }

        Ok(data.to_vec())
    }

    pub fn validate(&self, data: &[u8]) -> Result<()> {
        self.load(data)?;
        Ok(())
    }
}

impl Default for AotLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_valid_aot() {
        let loader = AotLoader::new();
        let mut data = vec![0x00, 0x61, 0x73, 0x6D];
        data.extend_from_slice(&[1, 0, 0, 0]);
        assert!(loader.validate(&data).is_ok());
    }
}
