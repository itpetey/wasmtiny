use crate::runtime::{Result, WasmError, WasmValue};

pub struct AotRuntime {
    native_functions: Vec<NativeFunc>,
}

type NativeFunc = Box<dyn Fn(&[WasmValue]) -> Result<Vec<WasmValue>> + Send + Sync>;

impl AotRuntime {
    pub fn new() -> Self {
        Self {
            native_functions: Vec::new(),
        }
    }

    pub fn register_native(&mut self, func: NativeFunc) -> u32 {
        let idx = self.native_functions.len() as u32;
        self.native_functions.push(func);
        idx
    }

    pub fn call_native(&self, idx: u32, args: &[WasmValue]) -> Result<Vec<WasmValue>> {
        let func = self
            .native_functions
            .get(idx as usize)
            .ok_or_else(|| WasmError::Runtime(format!("native function {} not found", idx)))?;
        func(args)
    }

    pub fn get_native_count(&self) -> usize {
        self.native_functions.len()
    }
}

impl Default for AotRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_registration() {
        let mut runtime = AotRuntime::new();
        let idx = runtime.register_native(Box::new(|_| Ok(vec![])));
        assert_eq!(idx, 0);
        assert_eq!(runtime.get_native_count(), 1);
    }
}
