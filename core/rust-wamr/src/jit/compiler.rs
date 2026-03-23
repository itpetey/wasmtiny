use crate::runtime::{Module, Result, WasmError, WasmValue};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq)]
pub enum CompilationTier {
    Baseline,
    Optimized,
}

#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub id: u64,
    pub tier: CompilationTier,
    pub code: Vec<u8>,
}

pub struct JitCompiler {
    code_cache: HashMap<u64, CompiledFunction>,
    compilation_tier: CompilationTier,
}

impl JitCompiler {
    pub fn new() -> Self {
        Self {
            code_cache: HashMap::new(),
            compilation_tier: CompilationTier::Baseline,
        }
    }

    pub fn compile(&mut self, module: &Module, func_idx: u32) -> Result<CompiledFunction> {
        let cache_key = self.compute_cache_key(module, func_idx);

        if let Some(cached) = self.code_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let func = module
            .func_at(func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))?;

        let code = self.translate_wasm_to_ir(&func.body);

        let compiled = CompiledFunction {
            id: func_idx as u64,
            tier: self.compilation_tier.clone(),
            code,
        };

        self.code_cache.insert(cache_key, compiled.clone());
        Ok(compiled)
    }

    fn translate_wasm_to_ir(&self, bytecode: &[u8]) -> Vec<u8> {
        let mut ir = Vec::new();
        let mut i = 0;

        while i < bytecode.len() {
            let opcode = bytecode[i];
            match opcode {
                0x20 => {
                    ir.push(0x01);
                    ir.push(bytecode[i + 1]);
                    i += 2;
                }
                0x21 => {
                    ir.push(0x02);
                    ir.push(bytecode[i + 1]);
                    i += 2;
                }
                0x6A => {
                    ir.push(0x10);
                    i += 1;
                }
                0x6B => {
                    ir.push(0x11);
                    i += 1;
                }
                0x6C => {
                    ir.push(0x12);
                    i += 1;
                }
                0x6D => {
                    ir.push(0x13);
                    i += 1;
                }
                0x0F => {
                    ir.push(0xFF);
                    i += 1;
                }
                _ => {
                    ir.push(0x00);
                    i += 1;
                }
            }
        }

        ir
    }

    fn compute_cache_key(&self, _module: &Module, func_idx: u32) -> u64 {
        func_idx as u64
    }

    pub fn set_tier(&mut self, tier: CompilationTier) {
        self.compilation_tier = tier;
    }

    pub fn clear_cache(&mut self) {
        self.code_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.code_cache.len()
    }

    pub fn get_compiled(&self, func_idx: u32) -> Option<&CompiledFunction> {
        self.code_cache.get(&(func_idx as u64))
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct JitRuntime {
    compiler: JitCompiler,
    compiled_code: HashMap<u64, *const ()>,
}

impl JitRuntime {
    pub fn new() -> Self {
        Self {
            compiler: JitCompiler::new(),
            compiled_code: HashMap::new(),
        }
    }

    pub fn compile_module(&mut self, module: &Module) -> Result<()> {
        for (idx, _) in module.funcs.iter().enumerate() {
            self.compile_function(module, idx as u32)?;
        }
        Ok(())
    }

    pub fn compile_function(&mut self, module: &Module, func_idx: u32) -> Result<CompiledFunction> {
        let compiled = self.compiler.compile(module, func_idx)?;

        if let Some(func) = module.func_at(func_idx) {
            self.compiled_code
                .insert(func_idx as u64, compiled.code.as_ptr() as *const ());
        }

        Ok(compiled)
    }

    pub fn execute(
        &self,
        module_idx: u32,
        func_idx: u32,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        let _ = (module_idx, func_idx, args);

        Ok(vec![WasmValue::I32(0)])
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
    fn test_compiler_creation() {
        let compiler = JitCompiler::new();
        assert_eq!(compiler.cache_size(), 0);
    }

    #[test]
    fn test_tier_switching() {
        let mut compiler = JitCompiler::new();
        compiler.set_tier(CompilationTier::Optimized);
        assert_eq!(compiler.compilation_tier, CompilationTier::Optimized);
    }

    #[test]
    fn test_jit_runtime() {
        let runtime = JitRuntime::new();
        assert_eq!(runtime.compiler.cache_size(), 0);
    }
}
