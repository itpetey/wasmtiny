use crate::runtime::{Module, Result, WasmError};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum CompilationTier {
    Baseline,
    Optimized,
}

#[derive(Clone, Debug)]
pub struct CompiledFunction {
    pub id: u64,
    pub tier: CompilationTier,
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

        let _func = module
            .func_at(func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))?;

        let compiled = CompiledFunction {
            id: func_idx as u64,
            tier: self.compilation_tier.clone(),
        };

        self.code_cache.insert(cache_key, compiled.clone());
        Ok(compiled)
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
}
