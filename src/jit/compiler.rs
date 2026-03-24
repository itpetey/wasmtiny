#[cfg(test)]
use crate::runtime::WasmValue;
use crate::runtime::{Module, Result, WasmError};
use std::collections::{HashMap, VecDeque};
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
    call_counts: HashMap<u64, u64>,
    osr_queue: OsrQueue,
    osr_compilation_queue: OsrCompilationQueue,
    osr_enabled: bool,
}

#[allow(clippy::new_without_default)]
impl JitCompiler {
    pub fn new() -> Self {
        Self {
            code_cache: HashMap::new(),
            compilation_tier: CompilationTier::Baseline,
            call_counts: HashMap::new(),
            osr_queue: OsrQueue::new(),
            osr_compilation_queue: OsrCompilationQueue::new(),
            osr_enabled: false,
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

    pub fn record_call(&mut self, func_idx: u64) {
        let count = self.call_counts.entry(func_idx).or_insert(0);
        *count += 1;
    }

    pub fn get_call_count(&self, func_idx: u64) -> u64 {
        *self.call_counts.get(&func_idx).unwrap_or(&0)
    }

    pub fn is_hot(&self, func_idx: u64) -> bool {
        self.get_call_count(func_idx) >= HOT_THRESHOLD
    }

    pub fn queue_for_osr(&mut self, func_idx: u64) {
        self.osr_queue.push(func_idx);
    }

    pub fn get_next_osr_candidate(&mut self) -> Option<u64> {
        self.osr_queue.pop()
    }

    pub fn has_osr_candidates(&self) -> bool {
        !self.osr_queue.is_empty()
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

    pub fn queue_osr_compilation(&mut self, func_idx: u64, module_id: u64, priority: u32) {
        let task = OsrCompilationTask {
            func_idx,
            module_id,
            priority,
        };
        self.osr_compilation_queue.push(task);
    }

    pub fn get_next_osr_task(&mut self) -> Option<OsrCompilationTask> {
        self.osr_compilation_queue.pop()
    }

    pub fn has_pending_osr_tasks(&self) -> bool {
        !self.osr_compilation_queue.is_empty()
    }

    pub fn compile_optimized(
        &mut self,
        module: &Module,
        func_idx: u32,
    ) -> Result<CompiledFunction> {
        let prev_tier = std::mem::replace(&mut self.compilation_tier, CompilationTier::Optimized);
        let result = self.compile(module, func_idx);
        self.compilation_tier = prev_tier;
        result
    }

    pub fn create_osr_entry_point(&self, _func_idx: u64, code: &[u8]) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(code);
        entry
    }
}

pub const HOT_THRESHOLD: u64 = 1000;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OsrFrameMetadata {
    pub func_idx: u64,
    pub pc: u32,
    pub locals: Vec<OsrValue>,
    pub operand_stack: Vec<OsrValue>,
    pub control_frames: Vec<OsrControlFrame>,
}

#[allow(dead_code)]
impl OsrFrameMetadata {
    pub fn new(func_idx: u64, pc: u32) -> Self {
        Self {
            func_idx,
            pc,
            locals: Vec::new(),
            operand_stack: Vec::new(),
            control_frames: Vec::new(),
        }
    }

    pub fn with_locals(mut self, locals: Vec<OsrValue>) -> Self {
        self.locals = locals;
        self
    }

    pub fn with_operand_stack(mut self, stack: Vec<OsrValue>) -> Self {
        self.operand_stack = stack;
        self
    }

    pub fn with_control_frames(mut self, frames: Vec<OsrControlFrame>) -> Self {
        self.control_frames = frames;
        self
    }

    pub fn transfer_locals(&self, target: &mut [OsrValue]) {
        for (i, val) in self.locals.iter().enumerate() {
            if i < target.len() {
                target[i] = val.clone();
            }
        }
    }

    pub fn transfer_stack(&self, target: &mut Vec<OsrValue>) {
        target.clear();
        target.extend_from_slice(&self.operand_stack);
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum OsrValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Ref(Option<u64>),
}

#[allow(dead_code)]
impl OsrValue {
    pub fn from_wasm_value(wasm: crate::runtime::WasmValue) -> Self {
        match wasm {
            crate::runtime::WasmValue::I32(v) => OsrValue::I32(v),
            crate::runtime::WasmValue::I64(v) => OsrValue::I64(v),
            crate::runtime::WasmValue::F32(v) => OsrValue::F32(v),
            crate::runtime::WasmValue::F64(v) => OsrValue::F64(v),
            crate::runtime::WasmValue::FuncRef(idx) => OsrValue::Ref(Some(idx as u64)),
            crate::runtime::WasmValue::ExternRef(idx) => OsrValue::Ref(Some(idx as u64)),
            crate::runtime::WasmValue::NullRef(_) => OsrValue::Ref(None),
        }
    }

    pub fn to_wasm_value(&self) -> crate::runtime::WasmValue {
        match self {
            OsrValue::I32(v) => crate::runtime::WasmValue::I32(*v),
            OsrValue::I64(v) => crate::runtime::WasmValue::I64(*v),
            OsrValue::F32(v) => crate::runtime::WasmValue::F32(*v),
            OsrValue::F64(v) => crate::runtime::WasmValue::F64(*v),
            OsrValue::Ref(idx) => crate::runtime::WasmValue::FuncRef(idx.unwrap_or(0) as u32),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OsrControlFrame {
    pub block_type: u32,
    pub start_pc: u32,
    pub end_pc: u32,
}

#[allow(dead_code)]
pub struct OsrContext {
    pub func_idx: u64,
    pub call_count: u64,
    pub state: OsrState,
    pub frame_metadata: Option<OsrFrameMetadata>,
}

#[allow(dead_code)]
impl OsrContext {
    pub fn new(func_idx: u64, call_count: u64) -> Self {
        Self {
            func_idx,
            call_count,
            state: OsrState::Pending,
            frame_metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: OsrFrameMetadata) -> Self {
        self.frame_metadata = Some(metadata);
        self
    }

    pub fn extract_locals(&self) -> Vec<OsrValue> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.locals.clone())
            .unwrap_or_default()
    }

    pub fn extract_operand_stack(&self) -> Vec<OsrValue> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.operand_stack.clone())
            .unwrap_or_default()
    }

    pub fn extract_control_frames(&self) -> Vec<OsrControlFrame> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.control_frames.clone())
            .unwrap_or_default()
    }

    pub fn set_state(&mut self, state: OsrState) {
        self.state = state;
    }

    pub fn mark_ready(&mut self) {
        self.state = OsrState::Ready;
    }

    pub fn mark_transitioning(&mut self) {
        self.state = OsrState::Transitioning;
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum OsrState {
    Pending,
    Compiling,
    Ready,
    Transitioning,
}

pub struct OsrQueue {
    queue: VecDeque<u64>,
}

#[allow(dead_code)]
impl OsrQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, func_idx: u64) {
        if !self.queue.contains(&func_idx) {
            self.queue.push_back(func_idx);
        }
    }

    pub fn pop(&mut self) -> Option<u64> {
        self.queue.pop_front()
    }

    pub fn contains(&self, func_idx: u64) -> bool {
        self.queue.contains(&func_idx)
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for OsrQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct OsrCompilationTask {
    pub func_idx: u64,
    pub module_id: u64,
    pub priority: u32,
}

pub struct OsrCompilationQueue {
    tasks: Vec<OsrCompilationTask>,
}

#[allow(dead_code)]
impl OsrCompilationQueue {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn push(&mut self, task: OsrCompilationTask) {
        let pos = self
            .tasks
            .iter()
            .position(|t| t.priority > task.priority)
            .unwrap_or(self.tasks.len());
        self.tasks.insert(pos, task);
    }

    pub fn pop(&mut self) -> Option<OsrCompilationTask> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for OsrCompilationQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct OsrTrampoline {
    pub from_tier: CompilationTier,
    pub to_tier: CompilationTier,
    pub code: Vec<u8>,
    pub entry_points: Vec<OsrEntryPoint>,
}

#[derive(Clone, Debug)]
pub struct OsrEntryPoint {
    pub pc_offset: u32,
    pub target_address: u64,
}

#[allow(dead_code)]
impl OsrTrampoline {
    pub fn new(from_tier: CompilationTier, to_tier: CompilationTier) -> Self {
        Self {
            from_tier,
            to_tier,
            code: Vec::new(),
            entry_points: Vec::new(),
        }
    }

    pub fn add_entry_point(&mut self, pc_offset: u32, target_address: u64) {
        self.entry_points.push(OsrEntryPoint {
            pc_offset,
            target_address,
        });
    }

    pub fn patch_code(&self, code: &mut [u8], _target_func_idx: u64) -> Result<()> {
        for entry in &self.entry_points {
            if (entry.pc_offset as usize) < code.len() {
                let patch_offset = entry.pc_offset as usize;
                code[patch_offset] = 0xE9;
                let addr_bytes = entry.target_address.to_le_bytes();
                for (i, byte) in addr_bytes.iter().enumerate() {
                    if patch_offset + 1 + i < code.len() {
                        code[patch_offset + 1 + i] = *byte;
                    }
                }
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub struct OsrJumpBuffer {
    pub locals: Vec<OsrValue>,
    pub operand_stack: Vec<OsrValue>,
    pub pc: u32,
    pub func_idx: u64,
}

#[allow(dead_code)]
impl OsrJumpBuffer {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            operand_stack: Vec::new(),
            pc: 0,
            func_idx: 0,
        }
    }

    pub fn capture(&mut self, func_idx: u64, pc: u32, locals: Vec<OsrValue>, stack: Vec<OsrValue>) {
        self.func_idx = func_idx;
        self.pc = pc;
        self.locals = locals;
        self.operand_stack = stack;
    }

    pub fn restore(&self) -> (u32, Vec<OsrValue>, Vec<OsrValue>) {
        (self.pc, self.locals.clone(), self.operand_stack.clone())
    }

    pub fn transfer_to(&self, target_locals: &mut [OsrValue], target_stack: &mut Vec<OsrValue>) {
        for (i, val) in self.locals.iter().enumerate() {
            if i < target_locals.len() {
                target_locals[i] = val.clone();
            }
        }
        target_stack.clear();
        target_stack.extend_from_slice(&self.operand_stack);
    }
}

impl Default for OsrJumpBuffer {
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

    #[test]
    fn test_osr_queue_push_and_pop() {
        let mut queue = OsrQueue::new();
        assert!(queue.is_empty());

        queue.push(1);
        queue.push(2);
        assert_eq!(queue.len(), 2);

        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_osr_queue_no_duplicates() {
        let mut queue = OsrQueue::new();
        queue.push(1);
        queue.push(1);
        queue.push(2);
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_hot_threshold() {
        let mut compiler = JitCompiler::new();
        assert!(!compiler.is_hot(0));

        compiler.record_call(0);
        assert!(!compiler.is_hot(0));

        for _ in 0..998 {
            compiler.record_call(0);
        }
        assert!(!compiler.is_hot(0));

        compiler.record_call(0);
        assert!(compiler.is_hot(0));
    }

    #[test]
    fn test_osr_compilation_queue_priority() {
        let mut queue = OsrCompilationQueue::new();
        queue.push(OsrCompilationTask {
            func_idx: 1,
            module_id: 0,
            priority: 10,
        });
        queue.push(OsrCompilationTask {
            func_idx: 2,
            module_id: 0,
            priority: 5,
        });
        queue.push(OsrCompilationTask {
            func_idx: 3,
            module_id: 0,
            priority: 15,
        });

        assert_eq!(queue.len(), 3);

        let task1 = queue.pop().unwrap();
        assert_eq!(task1.priority, 5);

        let task2 = queue.pop().unwrap();
        assert_eq!(task2.priority, 10);

        let task3 = queue.pop().unwrap();
        assert_eq!(task3.priority, 15);

        assert!(queue.is_empty());
    }

    #[test]
    fn test_osr_context_creation() {
        let context = OsrContext::new(42, 1000);
        assert_eq!(context.func_idx, 42);
        assert_eq!(context.call_count, 1000);
        assert_eq!(context.state, OsrState::Pending);
    }

    #[test]
    fn test_osr_context_state_transitions() {
        let mut context = OsrContext::new(1, 100);
        context.mark_ready();
        assert_eq!(context.state, OsrState::Ready);

        context.mark_transitioning();
        assert_eq!(context.state, OsrState::Transitioning);
    }

    #[test]
    fn test_osr_frame_metadata() {
        let metadata = OsrFrameMetadata::new(1, 100);
        assert_eq!(metadata.func_idx, 1);
        assert_eq!(metadata.pc, 100);
        assert!(metadata.locals.is_empty());
        assert!(metadata.operand_stack.is_empty());

        let metadata = metadata.with_locals(vec![OsrValue::I32(42)]);
        assert_eq!(metadata.locals.len(), 1);
    }

    #[test]
    fn test_osr_value_conversion() {
        let wasm_val = WasmValue::I32(42);
        let osr_val = OsrValue::from_wasm_value(wasm_val);
        assert!(matches!(osr_val, OsrValue::I32(42)));

        let wasm_back = osr_val.to_wasm_value();
        assert!(matches!(wasm_back, WasmValue::I32(42)));
    }

    #[test]
    fn test_osr_jump_buffer_capture_restore() {
        let mut buffer = OsrJumpBuffer::new();
        buffer.capture(
            1,
            100,
            vec![OsrValue::I32(1), OsrValue::I32(2)],
            vec![OsrValue::I64(99)],
        );

        let (pc, locals, stack) = buffer.restore();
        assert_eq!(pc, 100);
        assert_eq!(locals.len(), 2);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_osr_jump_buffer_transfer() {
        let mut buffer = OsrJumpBuffer::new();
        buffer.capture(
            1,
            0,
            vec![OsrValue::I32(10), OsrValue::I32(20)],
            vec![OsrValue::I64(30)],
        );

        let mut target_locals = vec![OsrValue::I32(0); 2];
        let mut target_stack = Vec::new();
        buffer.transfer_to(&mut target_locals, &mut target_stack);

        assert!(matches!(target_locals[0], OsrValue::I32(10)));
        assert!(matches!(target_locals[1], OsrValue::I32(20)));
        assert!(matches!(target_stack[0], OsrValue::I64(30)));
    }
}
