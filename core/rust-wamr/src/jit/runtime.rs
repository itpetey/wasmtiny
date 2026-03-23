use crate::runtime::{Result, WasmError, WasmValue};
use std::collections::HashMap;

pub struct JitRuntime {
    trampolines: HashMap<u32, Trampoline>,
    compiled_code: HashMap<u64, Vec<u8>>,
}

struct Trampoline {
    code: Vec<u8>,
    target: u32,
}

impl JitRuntime {
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

    pub fn get_trampoline(&self, table_idx: u32) -> Option<&Trampoline> {
        self.trampolines.get(&table_idx)
    }
}

impl Default for JitRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = JitRuntime::new();
        assert!(runtime.get_compiled(0).is_none());
    }
}
