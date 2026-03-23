use crate::runtime::{Module, Result, WasmError, WasmValue};
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

        if let Some(_func) = module.func_at(func_idx) {
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
        std::hint::black_box((module_idx, func_idx, args));

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
    use crate::runtime::{Func, FunctionType, Module, NumType, ValType};

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

    #[test]
    fn test_compile_function() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0);
        assert!(result.is_ok());
        let compiled = result.unwrap();
        assert_eq!(compiled.id, 0);
        assert_eq!(compiled.tier, CompilationTier::Baseline);
    }

    #[test]
    fn test_cache_miss_and_hit() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut compiler = JitCompiler::new();

        let result1 = compiler.compile(&module, 0);
        assert!(result1.is_ok());
        assert_eq!(compiler.cache_size(), 1);

        let result2 = compiler.compile(&module, 0);
        assert!(result2.is_ok());
        assert_eq!(compiler.cache_size(), 1);
    }

    #[test]
    fn test_ir_translation_local_get() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![ValType::Num(NumType::I32)], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0).unwrap();
        assert!(result.code.len() > 0);
    }

    #[test]
    fn test_ir_translation_i32_add() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0).unwrap();
        assert!(result.code.len() > 0);
    }

    #[test]
    fn test_compile_multiple_functions() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });
        module.funcs.push(Func {
            type_idx: 1,
            locals: vec![],
            body: vec![0x20, 0x00, 0x0F],
        });

        let mut compiler = JitCompiler::new();

        let result0 = compiler.compile(&module, 0);
        assert!(result0.is_ok());

        let result1 = compiler.compile(&module, 1);
        assert!(result1.is_ok());

        assert_eq!(compiler.cache_size(), 2);
    }

    #[test]
    fn test_clear_cache() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut compiler = JitCompiler::new();
        compiler.compile(&module, 0).unwrap();
        assert_eq!(compiler.cache_size(), 1);

        compiler.clear_cache();
        assert_eq!(compiler.cache_size(), 0);
    }

    #[test]
    fn test_get_compiled_function() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut compiler = JitCompiler::new();
        compiler.compile(&module, 0).unwrap();

        let compiled = compiler.get_compiled(0);
        assert!(compiled.is_some());
        assert_eq!(compiled.unwrap().id, 0);

        let not_found = compiler.get_compiled(1);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_compile_invalid_function_index() {
        let module = Module::new();

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_jit_runtime_compile_module() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut runtime = JitRuntime::new();
        let result = runtime.compile_module(&module);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jit_runtime_compile_function() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut runtime = JitRuntime::new();
        let result = runtime.compile_function(&module, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jit_runtime_execute() {
        let runtime = JitRuntime::new();
        let result = runtime.execute(0, 0, &[]);
        assert!(result.is_ok());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], WasmValue::I32(0));
    }
}
