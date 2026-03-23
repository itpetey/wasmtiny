#[cfg(test)]
use crate::runtime::WasmValue;
use crate::runtime::{Module, Result, WasmError};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

        let func = Self::defined_func(module, func_idx)?;

        let code = self.translate_wasm_to_ir(&func.body)?;

        let compiled = CompiledFunction {
            id: func_idx as u64,
            tier: self.compilation_tier.clone(),
            code,
        };

        self.code_cache.insert(cache_key, compiled.clone());
        Ok(compiled)
    }

    fn translate_wasm_to_ir(&self, bytecode: &[u8]) -> Result<Vec<u8>> {
        let mut ir = Vec::new();
        let mut i = 0;

        while i < bytecode.len() {
            let opcode = bytecode[i];
            match opcode {
                0x20 => {
                    let mut cursor = i + 1;
                    let local_idx = Self::read_uleb(bytecode, &mut cursor)?;
                    ir.push(0x01);
                    ir.push(u8::try_from(local_idx).map_err(|_| {
                        WasmError::Runtime(format!(
                            "local.get index {} exceeds JIT operand width",
                            local_idx
                        ))
                    })?);
                    i = cursor;
                }
                0x21 => {
                    let mut cursor = i + 1;
                    let local_idx = Self::read_uleb(bytecode, &mut cursor)?;
                    ir.push(0x02);
                    ir.push(u8::try_from(local_idx).map_err(|_| {
                        WasmError::Runtime(format!(
                            "local.set index {} exceeds JIT operand width",
                            local_idx
                        ))
                    })?);
                    i = cursor;
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
                    return Err(WasmError::Runtime(format!(
                        "unsupported opcode in JIT compiler: {:02x}",
                        opcode
                    )));
                }
            }
        }

        Ok(ir)
    }

    fn compute_cache_key(&self, module: &Module, func_idx: u32) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.compilation_tier.hash(&mut hasher);
        func_idx.hash(&mut hasher);
        if let Ok(func) = Self::defined_func(module, func_idx) {
            func.type_idx.hash(&mut hasher);
            func.body.hash(&mut hasher);
            for local in &func.locals {
                local.count.hash(&mut hasher);
                local.type_.hash(&mut hasher);
            }
        }
        hasher.finish()
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

    pub fn get_compiled(&self, module: &Module, func_idx: u32) -> Option<&CompiledFunction> {
        let cache_key = self.compute_cache_key(module, func_idx);
        self.code_cache.get(&cache_key)
    }

    fn read_uleb(bytecode: &[u8], cursor: &mut usize) -> Result<u32> {
        let mut value = 0u32;
        let mut shift = 0u32;

        loop {
            let byte = *bytecode
                .get(*cursor)
                .ok_or_else(|| WasmError::Runtime("unexpected end of JIT immediate".to_string()))?;
            *cursor += 1;
            value |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Runtime(
                    "uleb128 overflow in JIT immediate".to_string(),
                ));
            }
        }
    }

    fn import_func_count(module: &Module) -> u32 {
        module
            .imports
            .iter()
            .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
            .count() as u32
    }

    fn defined_func(module: &Module, func_idx: u32) -> Result<&crate::runtime::Func> {
        let import_func_count = Self::import_func_count(module);
        if func_idx < import_func_count {
            return Err(WasmError::Runtime(format!(
                "cannot JIT-compile imported function {}",
                func_idx
            )));
        }

        let local_idx = func_idx - import_func_count;
        module
            .defined_func_at(local_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))
    }
}

impl Default for JitCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub struct JitRuntime {
    compiler: JitCompiler,
    compiled_code: HashMap<u64, Vec<u8>>,
}

#[cfg(test)]
impl JitRuntime {
    pub fn new() -> Self {
        Self {
            compiler: JitCompiler::new(),
            compiled_code: HashMap::new(),
        }
    }

    pub fn compile_module(&mut self, module: &Module) -> Result<()> {
        let import_func_count = JitCompiler::import_func_count(module);
        for (idx, _) in module.funcs.iter().enumerate() {
            self.compile_function(module, import_func_count + idx as u32)?;
        }
        Ok(())
    }

    pub fn compile_function(&mut self, module: &Module, func_idx: u32) -> Result<CompiledFunction> {
        let cache_key = self.compiler.compute_cache_key(module, func_idx);
        let compiled = self.compiler.compile(module, func_idx)?;

        if JitCompiler::defined_func(module, func_idx).is_ok() {
            self.compiled_code.insert(cache_key, compiled.code.clone());
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
        Err(WasmError::Runtime(
            "JIT execution is not implemented".to_string(),
        ))
    }
}

#[cfg(test)]
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
    fn test_ir_translation_decodes_multibyte_local_indices() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x81, 0x01, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0).unwrap();
        assert_eq!(result.code, vec![0x01, 129, 0xFF]);
    }

    #[test]
    fn test_compile_rejects_unsupported_opcode() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let error = compiler.compile(&module, 0).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("unsupported opcode"))
        );
    }

    #[test]
    fn test_compile_rejects_truncated_local_immediate() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20],
        });

        let mut compiler = JitCompiler::new();
        let error = compiler.compile(&module, 0).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("unexpected end of JIT immediate"))
        );
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

        let compiled = compiler.get_compiled(&module, 0);
        assert!(compiled.is_some());
        assert_eq!(compiled.unwrap().id, 0);

        let not_found = compiler.get_compiled(&module, 1);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_cache_key_distinguishes_modules() {
        let mut first = Module::new();
        first.types.push(FunctionType::new(vec![], vec![]));
        first.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x0F],
        });

        let mut second = Module::new();
        second.types.push(FunctionType::new(vec![], vec![]));
        second.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x21, 0x00, 0x0F],
        });

        let mut compiler = JitCompiler::new();
        let first_compiled = compiler.compile(&first, 0).unwrap();
        let second_compiled = compiler.compile(&second, 0).unwrap();

        assert_ne!(first_compiled.code, second_compiled.code);
        assert_eq!(compiler.cache_size(), 2);
    }

    #[test]
    fn test_cache_key_distinguishes_compilation_tiers() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut compiler = JitCompiler::new();
        let baseline = compiler.compile(&module, 0).unwrap();
        compiler.set_tier(CompilationTier::Optimized);
        let optimised = compiler.compile(&module, 0).unwrap();

        assert_eq!(baseline.tier, CompilationTier::Baseline);
        assert_eq!(optimised.tier, CompilationTier::Optimized);
        assert_eq!(compiler.cache_size(), 2);
    }

    #[test]
    fn test_compile_invalid_function_index() {
        let module = Module::new();

        let mut compiler = JitCompiler::new();
        let result = compiler.compile(&module, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_rejects_imported_function_index() {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });

        let mut compiler = JitCompiler::new();
        let error = compiler.compile(&module, 0).unwrap_err();
        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("imported function"))
        );
    }

    #[test]
    fn test_compile_uses_combined_function_index_space() {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut compiler = JitCompiler::new();
        let compiled = compiler.compile(&module, 1).unwrap();

        assert_eq!(compiled.id, 1);
        assert!(compiler.get_compiled(&module, 1).is_some());
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
    fn test_jit_runtime_compile_module_uses_combined_indices() {
        let mut module = Module::new();
        module.types.push(FunctionType::empty());
        module.imports.push(crate::runtime::Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: crate::runtime::ImportKind::Func(0),
        });
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0F],
        });

        let mut runtime = JitRuntime::new();
        runtime.compile_module(&module).unwrap();

        assert!(runtime.compiler.get_compiled(&module, 1).is_some());
    }

    #[test]
    fn test_jit_runtime_execute() {
        let runtime = JitRuntime::new();
        let result = runtime.execute(0, 0, &[]);
        assert!(result.is_err());
    }
}
