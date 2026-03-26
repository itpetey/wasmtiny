use crate::jit::emitter::{Address, Condition, Emitter, Reg};
use crate::jit::regalloc::LinearScanAllocator;
#[cfg(test)]
use crate::runtime::WasmValue;
use crate::runtime::{Module, Result, WasmError};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

const WASM_UNREACHABLE: u8 = 0x00;
const WASM_NOP: u8 = 0x01;
#[allow(dead_code)]
const WASM_BLOCK: u8 = 0x02;
#[allow(dead_code)]
const WASM_LOOP: u8 = 0x03;
#[allow(dead_code)]
const WASM_IF: u8 = 0x04;
#[allow(dead_code)]
const WASM_ELSE: u8 = 0x05;
#[allow(dead_code)]
const WASM_END: u8 = 0x0B;
#[allow(dead_code)]
const WASM_BR: u8 = 0x0C;
#[allow(dead_code)]
const WASM_BR_IF: u8 = 0x0D;
#[allow(dead_code)]
const WASM_BR_TABLE: u8 = 0x0E;
const WASM_RETURN: u8 = 0x0F;
#[allow(dead_code)]
const WASM_CALL: u8 = 0x10;
#[allow(dead_code)]
const WASM_CALL_INDIRECT: u8 = 0x11;
const WASM_DROP: u8 = 0x1A;
#[allow(dead_code)]
const WASM_SELECT: u8 = 0x1B;
const WASM_LOCAL_GET: u8 = 0x20;
const WASM_LOCAL_SET: u8 = 0x21;
const WASM_LOCAL_TEE: u8 = 0x22;
#[allow(dead_code)]
const WASM_GLOBAL_GET: u8 = 0x23;
#[allow(dead_code)]
const WASM_GLOBAL_SET: u8 = 0x24;
const WASM_I32_LOAD: u8 = 0x28;
const WASM_I64_LOAD: u8 = 0x29;
#[allow(dead_code)]
const WASM_F32_LOAD: u8 = 0x2A;
#[allow(dead_code)]
const WASM_F64_LOAD: u8 = 0x2B;
const WASM_I32_STORE: u8 = 0x36;
const WASM_I64_STORE: u8 = 0x37;
#[allow(dead_code)]
const WASM_F32_STORE: u8 = 0x38;
#[allow(dead_code)]
const WASM_F64_STORE: u8 = 0x39;
const WASM_I32_CONST: u8 = 0x41;
#[allow(dead_code)]
const WASM_I64_CONST: u8 = 0x42;
const WASM_I32_ADD: u8 = 0x6A;
const WASM_I32_SUB: u8 = 0x6B;
const WASM_I32_MUL: u8 = 0x6C;
const WASM_I32_DIV_S: u8 = 0x6D;
const WASM_I32_DIV_U: u8 = 0x6E;
const WASM_I32_REM_S: u8 = 0x6F;
const WASM_I32_REM_U: u8 = 0x70;
const WASM_I32_AND: u8 = 0x71;
const WASM_I32_OR: u8 = 0x72;
const WASM_I32_XOR: u8 = 0x73;
const WASM_I32_SHL: u8 = 0x74;
const WASM_I32_SHR_S: u8 = 0x75;
const WASM_I32_SHR_U: u8 = 0x76;
#[allow(dead_code)]
const WASM_I32_ROTL: u8 = 0x79;
#[allow(dead_code)]
const WASM_I32_ROTR: u8 = 0x7A;
const WASM_I32_EQZ: u8 = 0x45;
const WASM_I32_EQ: u8 = 0x46;
const WASM_I32_NE: u8 = 0x47;
const WASM_I32_LT_S: u8 = 0x48;
const WASM_I32_LT_U: u8 = 0x49;
const WASM_I32_GT_S: u8 = 0x4A;
const WASM_I32_GT_U: u8 = 0x4B;
const WASM_I32_LE_S: u8 = 0x4C;
const WASM_I32_LE_U: u8 = 0x4D;
const WASM_I32_GE_S: u8 = 0x4E;
const WASM_I32_GE_U: u8 = 0x4F;
const WASM_I64_EQZ: u8 = 0x50;
const WASM_I64_EQ: u8 = 0x51;
const WASM_I64_NE: u8 = 0x52;
const WASM_I64_LT_S: u8 = 0x53;
const WASM_I64_LT_U: u8 = 0x54;
const WASM_I64_GT_S: u8 = 0x55;
const WASM_I64_GT_U: u8 = 0x56;
const WASM_I64_LE_S: u8 = 0x57;
const WASM_I64_LE_U: u8 = 0x58;
const WASM_I64_GE_S: u8 = 0x59;
const WASM_I64_GE_U: u8 = 0x5A;
const WASM_I64_ADD: u8 = 0x7C;
const WASM_I64_SUB: u8 = 0x7D;
const WASM_I64_MUL: u8 = 0x7E;
const WASM_I64_DIV_S: u8 = 0x7F;
const WASM_I64_DIV_U: u8 = 0x80;
const WASM_I64_REM_S: u8 = 0x81;
const WASM_I64_REM_U: u8 = 0x82;
const WASM_I64_AND: u8 = 0x83;
const WASM_I64_OR: u8 = 0x84;
const WASM_I64_XOR: u8 = 0x85;
const WASM_I64_SHL: u8 = 0x86;
const WASM_I64_SHR_S: u8 = 0x87;
const WASM_I64_SHR_U: u8 = 0x88;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// JIT compilation tier.
pub enum CompilationTier {
    /// Fast baseline code generation.
    Baseline,
    /// Higher-cost optimised code generation.
    Optimized,
}

#[derive(Clone, Debug)]
/// Compiled machine code for a single WebAssembly function.
pub struct CompiledFunction {
    /// Function identifier within the compiler cache.
    pub id: u64,
    /// Compilation tier used to produce this code.
    pub tier: CompilationTier,
    /// Generated machine-code bytes.
    pub code: Vec<u8>,
}

/// WebAssembly JIT compiler.
pub struct JitCompiler {
    code_cache: HashMap<u64, CompiledFunction>,
    compilation_tier: CompilationTier,
    call_counts: HashMap<u64, u64>,
    osr_queue: OsrQueue,
    osr_compilation_queue: OsrCompilationQueue,
    osr_enabled: bool,
    memory_size: u32,
}

#[derive(Clone, Copy, Debug)]
enum BlockKind {
    /// Variant `Block`.
    Block,
    /// Variant `Loop`.
    Loop,
    /// Variant `If`.
    If,
}

#[derive(Clone, Debug)]
struct BlockInfo {
    id: usize,
    kind: BlockKind,
    #[allow(dead_code)]
    wasm_pc: usize,
    x64_start: usize,
    x64_else: Option<usize>,
}

#[derive(Clone, Debug)]
struct PatchSite {
    x64_pos: usize,
    target_block_id: usize,
    is_rel8: bool,
}

#[allow(clippy::new_without_default)]
impl JitCompiler {
    /// Creates a new `JitCompiler`.
    pub fn new() -> Self {
        Self {
            code_cache: HashMap::new(),
            compilation_tier: CompilationTier::Baseline,
            call_counts: HashMap::new(),
            osr_queue: OsrQueue::new(),
            osr_compilation_queue: OsrCompilationQueue::new(),
            osr_enabled: false,
            memory_size: 65536,
        }
    }

    /// Sets memory size.
    pub fn set_memory_size(&mut self, size: u32) {
        self.memory_size = size;
    }

    /// Compiles the selected function.
    pub fn compile(&mut self, module: &Module, func_idx: u32) -> Result<CompiledFunction> {
        let cache_key = self.compute_cache_key(module, func_idx);

        if let Some(cached) = self.code_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let func = Self::defined_func(module, func_idx)?;

        let code = self.translate_wasm_to_x64(&func.body, &func.locals)?;

        let compiled = CompiledFunction {
            id: func_idx as u64,
            tier: self.compilation_tier.clone(),
            code,
        };

        self.code_cache.insert(cache_key, compiled.clone());
        Ok(compiled)
    }

    fn translate_wasm_to_x64(
        &self,
        bytecode: &[u8],
        locals: &[crate::runtime::Local],
    ) -> Result<Vec<u8>> {
        let mut emitter = Emitter::new();
        let mut block_stack: Vec<BlockInfo> = Vec::new();
        let mut patch_sites: Vec<PatchSite> = Vec::new();
        let mut pc: usize = 0;
        let local_count: u32 = locals.iter().map(|l| l.count).sum();
        let mem_size = self.memory_size;

        let stack_size = ((local_count as usize * 8 + 8) / 16 * 16) as u8;
        emitter.emit_sub_rsp(stack_size.max(16));

        let mut operand_stack_depth: usize = 0;
        let mut next_block_id: usize = 0;

        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            match opcode {
                WASM_LOCAL_GET => {
                    pc += 1;
                    let local_idx = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = LinearScanAllocator::spill_slot_offset(local_idx);
                    let addr = Address::new(Reg::Rsp).with_displacement(offset);
                    emitter.emit_mov_rm(Reg::Rax, &addr);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth += 1;
                }
                WASM_LOCAL_SET => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime(
                            "stack underflow in local.set".to_string(),
                        ));
                    }
                    pc += 1;
                    let local_idx = Self::read_uleb(bytecode, &mut pc)?;
                    emitter.emit_pop(Reg::Rax);
                    let offset = LinearScanAllocator::spill_slot_offset(local_idx);
                    let addr = Address::new(Reg::Rsp).with_displacement(offset);
                    emitter.emit_mov_mr(&addr, Reg::Rax);
                    operand_stack_depth -= 1;
                }
                WASM_LOCAL_TEE => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime(
                            "stack underflow in local.tee".to_string(),
                        ));
                    }
                    pc += 1;
                    let local_idx = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = LinearScanAllocator::spill_slot_offset(local_idx);
                    let addr = Address::new(Reg::Rsp).with_displacement(offset);
                    emitter.emit_mov_mr(&addr, Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                }
                WASM_I32_LOAD => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.load".to_string(),
                        ));
                    }
                    pc += 1;
                    let _align = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = Self::read_uleb(bytecode, &mut pc)? as i32;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_add_ri(Reg::Rax, offset);
                    emitter.emit_cmp_ri(Reg::Rax, (mem_size - 4) as i32);
                    emitter.emit_jcc_rel8(Condition::BelowOrEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_mov_rm(Reg::Rax, &Address::new(Reg::Rax).with_displacement(0));
                    emitter.emit_push(Reg::Rax);
                }
                WASM_I64_LOAD => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.load".to_string(),
                        ));
                    }
                    pc += 1;
                    let _align = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = Self::read_uleb(bytecode, &mut pc)? as i32;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_add_ri(Reg::Rax, offset);
                    emitter.emit_cmp_ri(Reg::Rax, (mem_size - 8) as i32);
                    emitter.emit_jcc_rel8(Condition::BelowOrEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_mov_rm(Reg::Rax, &Address::new(Reg::Rax).with_displacement(0));
                    emitter.emit_push(Reg::Rax);
                }
                WASM_I32_STORE => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.store".to_string(),
                        ));
                    }
                    pc += 1;
                    let _align = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = Self::read_uleb(bytecode, &mut pc)? as i32;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_add_ri(Reg::Rcx, offset);
                    emitter.emit_cmp_ri(Reg::Rcx, (mem_size - 4) as i32);
                    emitter.emit_jcc_rel8(Condition::BelowOrEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_mov_mr(&Address::new(Reg::Rcx).with_displacement(0), Reg::Rax);
                    operand_stack_depth -= 2;
                }
                WASM_I64_STORE => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.store".to_string(),
                        ));
                    }
                    pc += 1;
                    let _align = Self::read_uleb(bytecode, &mut pc)?;
                    let offset = Self::read_uleb(bytecode, &mut pc)? as i32;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_add_ri(Reg::Rcx, offset);
                    emitter.emit_cmp_ri(Reg::Rcx, (mem_size - 8) as i32);
                    emitter.emit_jcc_rel8(Condition::BelowOrEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_mov_mr(&Address::new(Reg::Rcx).with_displacement(0), Reg::Rax);
                    operand_stack_depth -= 2;
                }
                WASM_I32_ADD => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.add".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_add_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_SUB => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.sub".to_string()));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_sub_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_MUL => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.mul".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_mul_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_DIV_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.div_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_cdq();
                    emitter.emit_div_i32(Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_DIV_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.div_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_xor_rr(Reg::Rdx, Reg::Rdx);
                    emitter.emit_div_u32(Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_REM_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.rem_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_cdq();
                    emitter.emit_div_i32(Reg::Rcx);
                    emitter.emit_push(Reg::Rdx);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_REM_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.rem_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_xor_rr(Reg::Rdx, Reg::Rdx);
                    emitter.emit_div_u32(Reg::Rcx);
                    emitter.emit_push(Reg::Rdx);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_AND | WASM_I32_OR | WASM_I32_XOR => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in bitwise op".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    match opcode {
                        WASM_I32_AND => emitter.emit_and_rr(Reg::Rax, Reg::Rcx),
                        WASM_I32_OR => emitter.emit_or_rr(Reg::Rax, Reg::Rcx),
                        WASM_I32_XOR => emitter.emit_xor_rr(Reg::Rax, Reg::Rcx),
                        _ => {}
                    }
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_SHL => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.shl".to_string()));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_shl_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_SHR_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.shr_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_sar_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_SHR_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.shr_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_shr_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_CONST => {
                    pc += 1;
                    let val = Self::read_uleb(bytecode, &mut pc)?;
                    emitter.emit_mov_ri32(Reg::Rax, val);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth += 1;
                }
                WASM_I32_EQZ => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime("stack underflow in i32.eqz".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_cmp_ri(Reg::Rax, 0);
                    emitter.emit_mov_ra(Reg::Rax, 0);
                    emitter.emit_mov_ri32(Reg::Rcx, 1);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 4);
                    emitter.emit_mov_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    pc += 1;
                }
                WASM_I32_EQ => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.eq".to_string()));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::Equal);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_NE => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i32.ne".to_string()));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::NotEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_LT_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.lt_s".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::LessSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_LT_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.lt_u".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::Below);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_GT_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.gt_s".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::GreaterSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_GT_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.gt_u".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::Above);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_LE_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.le_s".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::LessOrEqualSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_LE_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.le_u".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::BelowOrEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_GE_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.ge_s".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::GreaterOrEqualSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I32_GE_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i32.ge_u".to_string(),
                        ));
                    }
                    Self::emit_i32_compare(&mut emitter, Condition::AboveOrEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_ADD => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.add".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_add_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_SUB => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.sub".to_string()));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_sub_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_MUL => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.mul".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_mul_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_DIV_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.div_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_cqo();
                    emitter.emit_div_i64(Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_DIV_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.div_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_xor_rr(Reg::Rdx, Reg::Rdx);
                    emitter.emit_div_u64(Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_REM_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.rem_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_cqo();
                    emitter.emit_div_i64(Reg::Rcx);
                    emitter.emit_push(Reg::Rdx);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_REM_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.rem_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rcx, Reg::Rcx);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 2);
                    emitter.emit_int3();
                    emitter.emit_xor_rr(Reg::Rdx, Reg::Rdx);
                    emitter.emit_div_u64(Reg::Rcx);
                    emitter.emit_push(Reg::Rdx);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_AND | WASM_I64_OR | WASM_I64_XOR => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64 bitwise".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_pop(Reg::Rcx);
                    match opcode {
                        WASM_I64_AND => emitter.emit_and_rr(Reg::Rax, Reg::Rcx),
                        WASM_I64_OR => emitter.emit_or_rr(Reg::Rax, Reg::Rcx),
                        WASM_I64_XOR => emitter.emit_xor_rr(Reg::Rax, Reg::Rcx),
                        _ => {}
                    }
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_SHL => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.shl".to_string()));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_shl_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_SHR_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.shr_s".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_sar_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_SHR_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.shr_u".to_string(),
                        ));
                    }
                    emitter.emit_pop(Reg::Rcx);
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_shr_cl(Reg::Rax);
                    emitter.emit_push(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_EQZ => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime("stack underflow in i64.eqz".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rax, Reg::Rax);
                    emitter.emit_mov_ri32(Reg::Rax, 0);
                    emitter.emit_mov_ri32(Reg::Rcx, 1);
                    emitter.emit_jcc_rel8(Condition::NotEqual, 4);
                    emitter.emit_mov_rr(Reg::Rax, Reg::Rcx);
                    emitter.emit_push(Reg::Rax);
                    pc += 1;
                }
                WASM_I64_EQ => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.eq".to_string()));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::Equal);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_NE => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime("stack underflow in i64.ne".to_string()));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::NotEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_LT_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.lt_s".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::LessSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_LT_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.lt_u".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::Below);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_GT_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.gt_s".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::GreaterSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_GT_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.gt_u".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::Above);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_LE_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.le_s".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::LessOrEqualSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_LE_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.le_u".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::BelowOrEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_GE_S => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.ge_s".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::GreaterOrEqualSigned);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_I64_GE_U => {
                    if operand_stack_depth < 2 {
                        return Err(WasmError::Runtime(
                            "stack underflow in i64.ge_u".to_string(),
                        ));
                    }
                    Self::emit_i64_compare(&mut emitter, Condition::AboveOrEqual);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_NOP => {
                    pc += 1;
                }
                WASM_DROP => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime("stack underflow in drop".to_string()));
                    }
                    emitter.emit_pop(Reg::Rax);
                    operand_stack_depth -= 1;
                    pc += 1;
                }
                WASM_RETURN => {
                    emitter.emit_add_rsp(stack_size.max(16));
                    emitter.emit_ret();
                    pc += 1;
                }
                WASM_UNREACHABLE => {
                    emitter.emit_int3();
                    pc += 1;
                }
                WASM_BLOCK => {
                    pc += 1;
                    let _block_type = Self::read_uleb(bytecode, &mut pc)?;
                    let block_id = next_block_id;
                    next_block_id += 1;
                    block_stack.push(BlockInfo {
                        id: block_id,
                        kind: BlockKind::Block,
                        wasm_pc: pc,
                        x64_start: emitter.code().len(),
                        x64_else: None,
                    });
                }
                WASM_LOOP => {
                    pc += 1;
                    let _block_type = Self::read_uleb(bytecode, &mut pc)?;
                    let block_id = next_block_id;
                    next_block_id += 1;
                    block_stack.push(BlockInfo {
                        id: block_id,
                        kind: BlockKind::Loop,
                        wasm_pc: pc,
                        x64_start: emitter.code().len(),
                        x64_else: None,
                    });
                }
                WASM_IF => {
                    if operand_stack_depth == 0 {
                        return Err(WasmError::Runtime("stack underflow in if".to_string()));
                    }
                    operand_stack_depth -= 1;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rax, Reg::Rax);
                    pc += 1;
                    let _block_type = Self::read_uleb(bytecode, &mut pc)?;
                    let block_id = next_block_id;
                    next_block_id += 1;
                    block_stack.push(BlockInfo {
                        id: block_id,
                        kind: BlockKind::If,
                        wasm_pc: pc,
                        x64_start: emitter.code().len(),
                        x64_else: None,
                    });
                    patch_sites.push(PatchSite {
                        x64_pos: emitter.code().len(),
                        target_block_id: block_id,
                        is_rel8: true,
                    });
                    emitter.emit_jcc_rel8(Condition::Equal, 0);
                }
                WASM_ELSE => {
                    if let Some(block) = block_stack.last_mut() {
                        let block_id = block.id;
                        block.x64_else = Some(emitter.code().len());
                        patch_sites.push(PatchSite {
                            x64_pos: emitter.code().len(),
                            target_block_id: block_id,
                            is_rel8: true,
                        });
                        emitter.emit_jmp_rel8(0);
                    }
                    pc += 1;
                }
                WASM_END => {
                    pc += 1;
                    if let Some(block) = block_stack.pop() {
                        let x64_end = emitter.code().len();
                        for site in patch_sites.iter_mut() {
                            if site.target_block_id == block.id {
                                let offset = (x64_end as i32) - (site.x64_pos as i32);
                                if site.is_rel8 {
                                    let code = emitter.code_mut();
                                    code[site.x64_pos + 1] = (offset - 2) as i8 as u8;
                                } else {
                                    let code = emitter.code_mut();
                                    let offset_bytes = (offset - 5).to_le_bytes();
                                    code[site.x64_pos + 1..site.x64_pos + 5]
                                        .copy_from_slice(&offset_bytes);
                                }
                            }
                        }
                    }
                }
                WASM_BR => {
                    pc += 1;
                    let label_idx = Self::read_uleb(bytecode, &mut pc)? as usize;
                    if label_idx < block_stack.len() {
                        let block = &block_stack[block_stack.len() - 1 - label_idx];
                        match block.kind {
                            BlockKind::Loop => {
                                let offset =
                                    (block.x64_start as i32) - (emitter.code().len() as i32);
                                emitter.emit_jmp_rel32(offset - 5);
                            }
                            BlockKind::Block | BlockKind::If => {
                                patch_sites.push(PatchSite {
                                    x64_pos: emitter.code().len(),
                                    target_block_id: block.id,
                                    is_rel8: false,
                                });
                                emitter.emit_jmp_rel32(0);
                            }
                        }
                    }
                }
                WASM_BR_IF => {
                    pc += 1;
                    let label_idx = Self::read_uleb(bytecode, &mut pc)? as usize;
                    emitter.emit_pop(Reg::Rax);
                    emitter.emit_test_rr(Reg::Rax, Reg::Rax);
                    if label_idx < block_stack.len() {
                        let block = &block_stack[block_stack.len() - 1 - label_idx];
                        match block.kind {
                            BlockKind::Loop => {
                                let offset =
                                    (block.x64_start as i32) - (emitter.code().len() as i32);
                                emitter.emit_jcc_rel32(Condition::NotEqual, offset - 6);
                            }
                            BlockKind::Block | BlockKind::If => {
                                patch_sites.push(PatchSite {
                                    x64_pos: emitter.code().len(),
                                    target_block_id: block.id,
                                    is_rel8: false,
                                });
                                emitter.emit_jcc_rel32(Condition::NotEqual, 0);
                            }
                        }
                    }
                }
                _ => {
                    return Err(WasmError::Runtime(format!(
                        "unsupported opcode in JIT compiler: {:02x}",
                        opcode
                    )));
                }
            }
        }

        Ok(emitter.take_code())
    }

    fn emit_i32_compare(emitter: &mut Emitter, cond: Condition) {
        emitter.emit_pop(Reg::Rcx);
        emitter.emit_pop(Reg::Rax);
        emitter.emit_cmp_rr(Reg::Rax, Reg::Rcx);
        emitter.emit_mov_ri32(Reg::Rax, 1);
        emitter.emit_jcc_rel8(cond, 3);
        emitter.emit_mov_ri32(Reg::Rax, 0);
    }

    fn emit_i64_compare(emitter: &mut Emitter, cond: Condition) {
        emitter.emit_pop(Reg::Rcx);
        emitter.emit_pop(Reg::Rax);
        emitter.emit_cmp_rr(Reg::Rax, Reg::Rcx);
        emitter.emit_mov_ri32(Reg::Rax, 1);
        emitter.emit_jcc_rel8(cond, 3);
        emitter.emit_mov_ri32(Reg::Rax, 0);
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn find_else_offset(bytecode: &[u8], start: usize) -> usize {
        let mut depth = 0;
        let mut i = start;
        while i < bytecode.len() {
            let opcode = bytecode[i];
            match opcode {
                WASM_BLOCK | WASM_LOOP | WASM_IF => {
                    depth += 1;
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_ELSE if depth == 1 => return i,
                WASM_END if depth == 0 => return 0,
                WASM_END => {
                    depth -= 1;
                    i += 1;
                }
                WASM_BR | WASM_BR_IF => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_LOCAL_GET | WASM_LOCAL_SET | WASM_LOCAL_TEE => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_CONST => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_LOAD | WASM_I64_LOAD | WASM_F32_LOAD | WASM_F64_LOAD => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_STORE | WASM_I64_STORE | WASM_F32_STORE | WASM_F64_STORE => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                _ => i += 1,
            }
        }
        0
    }

    #[allow(dead_code)]
    fn find_block_end(bytecode: &[u8], start: usize) -> usize {
        let mut depth = 0;
        let mut i = start;
        while i < bytecode.len() {
            let opcode = bytecode[i];
            match opcode {
                WASM_BLOCK | WASM_LOOP | WASM_IF => {
                    depth += 1;
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_ELSE | WASM_END => {
                    if depth == 0 {
                        return i;
                    }
                    depth -= 1;
                    i += 1;
                }
                WASM_BR | WASM_BR_IF => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_LOCAL_GET | WASM_LOCAL_SET | WASM_LOCAL_TEE => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_CONST => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_LOAD | WASM_I64_LOAD | WASM_F32_LOAD | WASM_F64_LOAD => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                WASM_I32_STORE | WASM_I64_STORE | WASM_F32_STORE | WASM_F64_STORE => {
                    i += 1;
                    let _ = Self::read_uleb(bytecode, &mut i);
                    let _ = Self::read_uleb(bytecode, &mut i);
                }
                _ => i += 1,
            }
        }
        0
    }

    #[allow(dead_code)]
    fn uleb_size(bytecode: &[u8], start: usize) -> usize {
        let mut size = 0;
        let mut i = start;
        while i < bytecode.len() {
            size += 1;
            if bytecode[i] & 0x80 == 0 {
                break;
            }
            i += 1;
        }
        size
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

    /// Sets tier.
    pub fn set_tier(&mut self, tier: CompilationTier) {
        self.compilation_tier = tier;
    }

    /// Clears cache.
    pub fn clear_cache(&mut self) {
        self.code_cache.clear();
    }

    /// Returns the number of cached compiled functions.
    pub fn cache_size(&self) -> usize {
        self.code_cache.len()
    }

    /// Returns compiled.
    pub fn get_compiled(&self, module: &Module, func_idx: u32) -> Option<&CompiledFunction> {
        let cache_key = self.compute_cache_key(module, func_idx);
        self.code_cache.get(&cache_key)
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

    /// Records a function call for tiering and OSR decisions.
    pub fn record_call(&mut self, func_idx: u64) {
        let count = self.call_counts.entry(func_idx).or_insert(0);
        *count += 1;

        if self.osr_enabled && *count >= HOT_THRESHOLD && !self.has_osr_candidates() {
            self.queue_for_osr(func_idx);
        }
    }

    /// Returns call count.
    pub fn get_call_count(&self, func_idx: u64) -> u64 {
        *self.call_counts.get(&func_idx).unwrap_or(&0)
    }

    /// Returns whether hot.
    pub fn is_hot(&self, func_idx: u64) -> bool {
        self.get_call_count(func_idx) >= HOT_THRESHOLD
    }

    /// Returns whether the function should be considered for OSR.
    pub fn should_osr(&self, func_idx: u64) -> bool {
        self.osr_enabled && self.is_hot(func_idx)
    }

    /// Queues the function for on-stack replacement.
    pub fn queue_for_osr(&mut self, func_idx: u64) {
        self.osr_queue.push(func_idx);
    }

    /// Returns next osr candidate.
    pub fn get_next_osr_candidate(&mut self) -> Option<u64> {
        self.osr_queue.pop()
    }

    /// Returns whether this value has osr candidates.
    pub fn has_osr_candidates(&self) -> bool {
        !self.osr_queue.is_empty()
    }

    /// Enables osr.
    pub fn enable_osr(&mut self) {
        self.osr_enabled = true;
    }

    /// Disables osr.
    pub fn disable_osr(&mut self) {
        self.osr_enabled = false;
    }

    /// Returns whether osr enabled.
    pub fn is_osr_enabled(&self) -> bool {
        self.osr_enabled
    }

    /// Queues an optimised OSR compilation task.
    pub fn queue_osr_compilation(&mut self, func_idx: u64, module_id: u64, priority: u32) {
        let task = OsrCompilationTask {
            func_idx,
            module_id,
            priority,
        };
        self.osr_compilation_queue.push(task);
    }

    /// Returns next osr task.
    pub fn get_next_osr_task(&mut self) -> Option<OsrCompilationTask> {
        self.osr_compilation_queue.pop()
    }

    /// Returns whether this value has pending osr tasks.
    pub fn has_pending_osr_tasks(&self) -> bool {
        !self.osr_compilation_queue.is_empty()
    }

    /// Compiles an optimised version of the selected function.
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

    /// Creates osr entry point.
    pub fn create_osr_entry_point(&self, _func_idx: u64, code: &[u8]) -> Vec<u8> {
        let mut entry = Vec::new();
        entry.extend_from_slice(code);
        entry
    }
}

/// Constant `HOT_THRESHOLD`.
pub const HOT_THRESHOLD: u64 = 1000;

#[derive(Clone, Debug)]
#[allow(dead_code)]
/// Captured frame metadata used for on-stack replacement.
pub struct OsrFrameMetadata {
    /// Function index associated with the captured frame.
    pub func_idx: u64,
    /// Program-counter offset within the function.
    pub pc: u32,
    /// The local values.
    pub locals: Vec<OsrValue>,
    /// Operand-stack values captured at the transition point.
    pub operand_stack: Vec<OsrValue>,
    /// Control-flow frames captured at the transition point.
    pub control_frames: Vec<OsrControlFrame>,
}

#[allow(dead_code)]
impl OsrFrameMetadata {
    /// Creates a new `OsrFrameMetadata`.
    pub fn new(func_idx: u64, pc: u32) -> Self {
        Self {
            func_idx,
            pc,
            locals: Vec::new(),
            operand_stack: Vec::new(),
            control_frames: Vec::new(),
        }
    }

    /// Returns this value configured with locals.
    pub fn with_locals(mut self, locals: Vec<OsrValue>) -> Self {
        self.locals = locals;
        self
    }

    /// Returns this value configured with operand stack.
    pub fn with_operand_stack(mut self, stack: Vec<OsrValue>) -> Self {
        self.operand_stack = stack;
        self
    }

    /// Returns this value configured with control frames.
    pub fn with_control_frames(mut self, frames: Vec<OsrControlFrame>) -> Self {
        self.control_frames = frames;
        self
    }

    /// Copies captured local values into the target storage.
    pub fn transfer_locals(&self, target: &mut [OsrValue]) {
        for (i, val) in self.locals.iter().enumerate() {
            if i < target.len() {
                target[i] = val.clone();
            }
        }
    }

    /// Copies captured stack values into the target storage.
    pub fn transfer_stack(&self, target: &mut Vec<OsrValue>) {
        target.clear();
        target.extend_from_slice(&self.operand_stack);
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
/// A value carried through an on-stack replacement transition.
pub enum OsrValue {
    /// A 32-bit integer value.
    I32(i32),
    /// A 64-bit integer value.
    I64(i64),
    /// A 32-bit floating-point value.
    F32(f32),
    /// A 64-bit floating-point value.
    F64(f64),
    /// A nullable reference encoded as an optional raw handle.
    Ref(Option<u64>),
}

#[allow(dead_code)]
impl OsrValue {
    /// Converts a WebAssembly value into its OSR representation.
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

    /// Converts this OSR value back into a WebAssembly value.
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
/// Control-frame metadata captured for on-stack replacement.
pub struct OsrControlFrame {
    /// Encoded block type for the frame.
    pub block_type: u32,
    /// Start program-counter offset for the frame.
    pub start_pc: u32,
    /// End program-counter offset for the frame.
    pub end_pc: u32,
}

#[allow(dead_code)]
/// Runtime state tracked for a function considered for OSR.
pub struct OsrContext {
    /// Function index associated with the context.
    pub func_idx: u64,
    /// Number of observed calls for the function.
    pub call_count: u64,
    /// Current OSR state for the function.
    pub state: OsrState,
    /// Captured frame metadata, when available.
    pub frame_metadata: Option<OsrFrameMetadata>,
}

#[allow(dead_code)]
impl OsrContext {
    /// Creates a new `OsrContext`.
    pub fn new(func_idx: u64, call_count: u64) -> Self {
        Self {
            func_idx,
            call_count,
            state: OsrState::Pending,
            frame_metadata: None,
        }
    }

    /// Returns this value configured with metadata.
    pub fn with_metadata(mut self, metadata: OsrFrameMetadata) -> Self {
        self.frame_metadata = Some(metadata);
        self
    }

    /// Extracts captured local values.
    pub fn extract_locals(&self) -> Vec<OsrValue> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.locals.clone())
            .unwrap_or_default()
    }

    /// Extracts the captured operand stack.
    pub fn extract_operand_stack(&self) -> Vec<OsrValue> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.operand_stack.clone())
            .unwrap_or_default()
    }

    /// Extracts the captured control frames.
    pub fn extract_control_frames(&self) -> Vec<OsrControlFrame> {
        self.frame_metadata
            .as_ref()
            .map(|m| m.control_frames.clone())
            .unwrap_or_default()
    }

    /// Sets state.
    pub fn set_state(&mut self, state: OsrState) {
        self.state = state;
    }

    /// Marks the transition as ready.
    pub fn mark_ready(&mut self) {
        self.state = OsrState::Ready;
    }

    /// Marks the transition as in progress.
    pub fn mark_transitioning(&mut self) {
        self.state = OsrState::Transitioning;
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
/// Lifecycle state for an OSR candidate.
pub enum OsrState {
    /// The function is queued but not yet being compiled.
    Pending,
    /// An optimised version is currently being compiled.
    Compiling,
    /// An optimised version is ready for transition.
    Ready,
    /// Execution is actively transitioning into optimised code.
    Transitioning,
}

/// Osr queue.
pub struct OsrQueue {
    queue: VecDeque<u64>,
}

#[allow(dead_code)]
impl OsrQueue {
    /// Creates a new `OsrQueue`.
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Pushes a value onto the stack.
    pub fn push(&mut self, func_idx: u64) {
        if !self.queue.contains(&func_idx) {
            self.queue.push_back(func_idx);
        }
    }

    /// Pops and returns the top value, if present.
    pub fn pop(&mut self) -> Option<u64> {
        self.queue.pop_front()
    }

    /// Returns whether the collection contains the given item.
    pub fn contains(&self, func_idx: u64) -> bool {
        self.queue.contains(&func_idx)
    }

    /// Returns the length.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns `true` if this value is empty.
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
/// A queued request to compile an OSR target.
pub struct OsrCompilationTask {
    /// Function index to compile.
    pub func_idx: u64,
    /// Module identifier used to resolve the function.
    pub module_id: u64,
    /// Scheduling priority for the task.
    pub priority: u32,
}

/// Osr compilation queue.
pub struct OsrCompilationQueue {
    tasks: Vec<OsrCompilationTask>,
}

#[allow(dead_code)]
impl OsrCompilationQueue {
    /// Creates a new `OsrCompilationQueue`.
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    /// Pushes a value onto the stack.
    pub fn push(&mut self, task: OsrCompilationTask) {
        let pos = self
            .tasks
            .iter()
            .position(|t| t.priority > task.priority)
            .unwrap_or(self.tasks.len());
        self.tasks.insert(pos, task);
    }

    /// Pops and returns the top value, if present.
    pub fn pop(&mut self) -> Option<OsrCompilationTask> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }

    /// Returns the length.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Returns `true` if this value is empty.
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
/// Transition stub used to enter optimised code during OSR.
pub struct OsrTrampoline {
    /// Source compilation tier for the transition.
    pub from_tier: CompilationTier,
    /// Destination compilation tier for the transition.
    pub to_tier: CompilationTier,
    /// The encoded bytes.
    pub code: Vec<u8>,
    /// OSR entry points available in the generated code.
    pub entry_points: Vec<OsrEntryPoint>,
}

#[derive(Clone, Debug)]
/// An entry point into optimised code for a specific program-counter offset.
pub struct OsrEntryPoint {
    /// WebAssembly program-counter offset for the transition site.
    pub pc_offset: u32,
    /// Native target address for the transition.
    pub target_address: u64,
}

#[allow(dead_code)]
impl OsrTrampoline {
    /// Creates a new `OsrTrampoline`.
    pub fn new(from_tier: CompilationTier, to_tier: CompilationTier) -> Self {
        Self {
            from_tier,
            to_tier,
            code: Vec::new(),
            entry_points: Vec::new(),
        }
    }

    /// Adds entry point.
    pub fn add_entry_point(&mut self, pc_offset: u32, target_address: u64) {
        self.entry_points.push(OsrEntryPoint {
            pc_offset,
            target_address,
        });
    }

    /// Patches compiled code for the requested transition.
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
/// Osr jump buffer.
pub struct OsrJumpBuffer {
    /// The local values.
    pub locals: Vec<OsrValue>,
    /// Operand-stack values captured for the jump.
    pub operand_stack: Vec<OsrValue>,
    /// Program-counter offset to resume from.
    pub pc: u32,
    /// Function index associated with the buffer.
    pub func_idx: u64,
}

#[allow(dead_code)]
impl OsrJumpBuffer {
    /// Creates a new `OsrJumpBuffer`.
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            operand_stack: Vec::new(),
            pc: 0,
            func_idx: 0,
        }
    }

    /// Captures the current execution state.
    pub fn capture(&mut self, func_idx: u64, pc: u32, locals: Vec<OsrValue>, stack: Vec<OsrValue>) {
        self.func_idx = func_idx;
        self.pc = pc;
        self.locals = locals;
        self.operand_stack = stack;
    }

    /// Restores the captured execution state.
    pub fn restore(&self) -> (u32, Vec<OsrValue>, Vec<OsrValue>) {
        (self.pc, self.locals.clone(), self.operand_stack.clone())
    }

    /// Transfers the captured OSR state into the target storage.
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
/// Jit runtime.
pub struct JitRuntime {
    compiler: JitCompiler,
    compiled_code: HashMap<u64, Vec<u8>>,
}

#[cfg(test)]
impl JitRuntime {
    /// Creates a new `JitRuntime`.
    pub fn new() -> Self {
        Self {
            compiler: JitCompiler::new(),
            compiled_code: HashMap::new(),
        }
    }

    /// Compiles all supported functions in the module.
    pub fn compile_module(&mut self, module: &Module) -> Result<()> {
        let import_func_count = JitCompiler::import_func_count(module);
        for (idx, _) in module.funcs.iter().enumerate() {
            self.compile_function(module, import_func_count + idx as u32)?;
        }
        Ok(())
    }

    /// Compiles a single function from the module.
    pub fn compile_function(&mut self, module: &Module, func_idx: u32) -> Result<CompiledFunction> {
        let cache_key = self.compiler.compute_cache_key(module, func_idx);
        let compiled = self.compiler.compile(module, func_idx)?;

        if JitCompiler::defined_func(module, func_idx).is_ok() {
            self.compiled_code.insert(cache_key, compiled.code.clone());
        }

        Ok(compiled)
    }

    /// Executes the requested function.
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
        assert!(!result.code.is_empty());
        assert!(result.code.contains(&0xC3));
    }

    #[test]
    fn test_compile_rejects_unsupported_opcode() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0xFC, 0x00],
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
            body: vec![0x41, 0x01, 0x0F],
        });

        let mut second = Module::new();
        second.types.push(FunctionType::new(vec![], vec![]));
        second.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x02, 0x0F],
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
    fn test_should_osr() {
        let mut compiler = JitCompiler::new();
        compiler.enable_osr();

        compiler.record_call(0);
        assert!(!compiler.should_osr(0));

        for _ in 0..998 {
            compiler.record_call(0);
        }
        assert!(!compiler.should_osr(0));

        compiler.record_call(0);
        assert!(compiler.should_osr(0));
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
