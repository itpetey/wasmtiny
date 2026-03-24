use std::collections::HashMap;

pub struct JitCodeCache {
    trampolines: HashMap<u32, Trampoline>,
    compiled_code: HashMap<u64, Vec<u8>>,
    osr_enabled: bool,
    osr_threshold: u64,
    osr_entry_points: HashMap<u64, Vec<OsrEntryPoint>>,
}

#[derive(Clone, Debug)]
pub struct OsrEntryPoint {
    pub pc_offset: u32,
    pub target_address: u64,
}

pub(crate) struct Trampoline {
    #[allow(dead_code)]
    code: Vec<u8>,
    #[allow(dead_code)]
    target: u32,
}

impl JitCodeCache {
    pub fn new() -> Self {
        Self {
            trampolines: HashMap::new(),
            compiled_code: HashMap::new(),
            osr_enabled: false,
            osr_threshold: 1000,
            osr_entry_points: HashMap::new(),
        }
    }

    pub fn enable_osr(&mut self) {
        self.osr_enabled = true;
    }

    pub fn disable_osr(&mut self) {
        self.osr_enabled = false;
    }

    pub fn is_osr_enabled(&self) -> bool {
        self.osr_enabled
    }

    pub fn set_osr_threshold(&mut self, threshold: u64) {
        self.osr_threshold = threshold;
    }

    pub fn get_osr_threshold(&self) -> u64 {
        self.osr_threshold
    }

    pub fn add_osr_entry_point(&mut self, func_idx: u64, pc_offset: u32, target_address: u64) {
        let entry = OsrEntryPoint {
            pc_offset,
            target_address,
        };
        self.osr_entry_points
            .entry(func_idx)
            .or_default()
            .push(entry);
    }

    pub fn get_osr_entry_points(&self, func_idx: u64) -> Option<&Vec<OsrEntryPoint>> {
        self.osr_entry_points.get(&func_idx)
    }

    pub fn add_trampoline(&mut self, table_idx: u32, target: u32) {
        let trampoline = Trampoline {
            code: vec![],
            target,
        };
        self.trampolines.insert(table_idx, trampoline);
    }

    pub fn store_compiled(&mut self, key: u64, code: Vec<u8>) {
        self.compiled_code.insert(key, code);
    }

    pub fn get_compiled(&self, key: u64) -> Option<&Vec<u8>> {
        self.compiled_code.get(&key)
    }

    #[allow(dead_code)]
    pub(crate) fn get_trampoline(&self, table_idx: u32) -> Option<&Trampoline> {
        self.trampolines.get(&table_idx)
    }
}

impl Default for JitCodeCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = JitCodeCache::new();
        assert!(runtime.get_compiled(0).is_none());
    }

    #[test]
    fn test_osr_configuration() {
        let mut cache = JitCodeCache::new();
        assert!(!cache.is_osr_enabled());
        assert_eq!(cache.get_osr_threshold(), 1000);

        cache.enable_osr();
        assert!(cache.is_osr_enabled());

        cache.set_osr_threshold(500);
        assert_eq!(cache.get_osr_threshold(), 500);

        cache.disable_osr();
        assert!(!cache.is_osr_enabled());
    }

    #[test]
    fn test_osr_entry_points() {
        let mut cache = JitCodeCache::new();
        cache.add_osr_entry_point(1, 100, 0x1000);
        cache.add_osr_entry_point(1, 200, 0x2000);

        let entries = cache.get_osr_entry_points(1);
        assert!(entries.is_some());
        assert_eq!(entries.unwrap().len(), 2);

        assert!(cache.get_osr_entry_points(999).is_none());
    }
}
