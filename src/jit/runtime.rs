use std::collections::HashMap;

pub struct JitCodeCache {
    trampolines: HashMap<u32, Trampoline>,
    compiled_code: HashMap<u64, Vec<u8>>,
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
        }
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
}
