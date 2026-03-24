#![allow(unsafe_op_in_unsafe_fn)]

use crate::runtime::{Func, Module, NumType, Result, ValType, WasmError};
use std::collections::HashMap;

#[cfg(feature = "llvm-jit")]
use llvm_sys::LLVMIntPredicate;
#[cfg(feature = "llvm-jit")]
use llvm_sys::LLVMRealPredicate;
#[cfg(feature = "llvm-jit")]
use llvm_sys::core::*;
#[cfg(feature = "llvm-jit")]
use llvm_sys::prelude::*;

#[cfg(feature = "llvm-jit")]
#[allow(dead_code)]
mod opcodes {
    pub const OP_UNREACHABLE: u8 = 0x00;
    pub const OP_NOP: u8 = 0x01;
    pub const OP_BLOCK: u8 = 0x02;
    pub const OP_LOOP: u8 = 0x03;
    pub const OP_IF: u8 = 0x04;
    pub const OP_ELSE: u8 = 0x05;
    pub const OP_END: u8 = 0x0B;
    pub const OP_BR: u8 = 0x0C;
    pub const OP_BR_IF: u8 = 0x0D;
    pub const OP_RETURN: u8 = 0x0F;
    pub const OP_CALL: u8 = 0x10;
    pub const OP_CALL_INDIRECT: u8 = 0x11;
    pub const OP_DROP: u8 = 0x1A;
    pub const OP_LOCAL_GET: u8 = 0x20;
    pub const OP_LOCAL_SET: u8 = 0x21;
    pub const OP_LOCAL_TEE: u8 = 0x22;
    pub const OP_I32_LOAD: u8 = 0x28;
    pub const OP_I64_LOAD: u8 = 0x29;
    pub const OP_F32_LOAD: u8 = 0x2A;
    pub const OP_F64_LOAD: u8 = 0x2B;
    pub const OP_I32_LOAD8_S: u8 = 0x2C;
    pub const OP_I32_LOAD8_U: u8 = 0x2D;
    pub const OP_I32_LOAD16_S: u8 = 0x2E;
    pub const OP_I32_LOAD16_U: u8 = 0x2F;
    pub const OP_I32_STORE: u8 = 0x36;
    pub const OP_I64_STORE: u8 = 0x37;
    pub const OP_F32_STORE: u8 = 0x38;
    pub const OP_F64_STORE: u8 = 0x39;
    pub const OP_I32_STORE8: u8 = 0x3A;
    pub const OP_I32_STORE16: u8 = 0x3B;
    pub const OP_I32_EQZ: u8 = 0x45;
    pub const OP_I32_EQ: u8 = 0x46;
    pub const OP_I32_NE: u8 = 0x47;
    pub const OP_I32_LT_S: u8 = 0x48;
    pub const OP_I32_LT_U: u8 = 0x49;
    pub const OP_I32_GT_S: u8 = 0x4A;
    pub const OP_I32_GT_U: u8 = 0x4B;
    pub const OP_I32_LE_S: u8 = 0x4C;
    pub const OP_I32_LE_U: u8 = 0x4D;
    pub const OP_I32_GE_S: u8 = 0x4E;
    pub const OP_I32_GE_U: u8 = 0x4F;
    pub const OP_I32_CONST: u8 = 0x41;
    pub const OP_I64_CONST: u8 = 0x42;
    pub const OP_F32_CONST: u8 = 0x43;
    pub const OP_F64_CONST: u8 = 0x44;
    pub const OP_I32_CLZ: u8 = 0x67;
    pub const OP_I32_CTZ: u8 = 0x68;
    pub const OP_I32_POPCNT: u8 = 0x69;
    pub const OP_I32_ADD: u8 = 0x6A;
    pub const OP_I32_SUB: u8 = 0x6B;
    pub const OP_I32_MUL: u8 = 0x6C;
    pub const OP_I32_DIV_S: u8 = 0x6D;
    pub const OP_I32_DIV_U: u8 = 0x6E;
    pub const OP_I32_REM_S: u8 = 0x6F;
    pub const OP_I32_REM_U: u8 = 0x70;
    pub const OP_I32_AND: u8 = 0x71;
    pub const OP_I32_OR: u8 = 0x72;
    pub const OP_I32_XOR: u8 = 0x73;
    pub const OP_I32_SHL: u8 = 0x74;
    pub const OP_I32_SHR_S: u8 = 0x75;
    pub const OP_I32_SHR_U: u8 = 0x76;
    pub const OP_I32_ROTL: u8 = 0x77;
    pub const OP_I32_ROTR: u8 = 0x78;
    pub const OP_I64_CLZ: u8 = 0x79;
    pub const OP_I64_CTZ: u8 = 0x7A;
    pub const OP_I64_POPCNT: u8 = 0x7B;
    pub const OP_I64_ADD: u8 = 0x7C;
    pub const OP_I64_SUB: u8 = 0x7D;
    pub const OP_I64_MUL: u8 = 0x7E;
    pub const OP_I64_DIV_S: u8 = 0x7F;
    pub const OP_I64_DIV_U: u8 = 0x80;
    pub const OP_I64_REM_S: u8 = 0x81;
    pub const OP_I64_REM_U: u8 = 0x82;
    pub const OP_I64_AND: u8 = 0x83;
    pub const OP_I64_OR: u8 = 0x84;
    pub const OP_I64_XOR: u8 = 0x85;
    pub const OP_I64_SHL: u8 = 0x86;
    pub const OP_I64_SHR_S: u8 = 0x87;
    pub const OP_I64_SHR_U: u8 = 0x88;
    pub const OP_I64_ROTL: u8 = 0x89;
    pub const OP_I64_ROTR: u8 = 0x8A;
    pub const OP_F32_ADD: u8 = 0x8C;
    pub const OP_F32_SUB: u8 = 0x8D;
    pub const OP_F32_MUL: u8 = 0x8E;
    pub const OP_F32_DIV: u8 = 0x8F;
    pub const OP_F32_MIN: u8 = 0x90;
    pub const OP_F32_NEG: u8 = 0x91;
    pub const OP_F64_ADD: u8 = 0x92;
    pub const OP_F64_SUB: u8 = 0x93;
    pub const OP_F64_MUL: u8 = 0x94;
    pub const OP_F64_DIV: u8 = 0x95;
    pub const OP_F64_MIN: u8 = 0x96;
    pub const OP_F64_NEG: u8 = 0x97;
    pub const OP_F64_DEMOTE_F32: u8 = 0x98;
    pub const OP_F32_PROMOTE_F64: u8 = 0x99;
    pub const OP_I32_TRUNC_F32_S: u8 = 0xA2;
    pub const OP_I32_TRUNC_F32_U: u8 = 0xA3;
    pub const OP_I64_TRUNC_F32_S: u8 = 0xA4;
    pub const OP_I64_TRUNC_F32_U: u8 = 0xA5;
    pub const OP_I32_TRUNC_F64_S: u8 = 0xA6;
    pub const OP_F32_CONVERT_I32_S: u8 = 0xC0;
    pub const OP_F32_CONVERT_I32_U: u8 = 0xC1;
    pub const OP_F32_CONVERT_I64_S: u8 = 0xC2;
    pub const OP_F32_CONVERT_I64_U: u8 = 0xC3;
    pub const OP_F64_CONVERT_I32_S: u8 = 0xC4;
    pub const OP_F64_CONVERT_I32_U: u8 = 0xC5;
    pub const OP_F64_CONVERT_I64_S: u8 = 0xC6;
    pub const OP_F64_CONVERT_I64_U: u8 = 0xC7;
    pub const OP_I32_REINTERPRET_F32: u8 = 0xBC;
    pub const OP_I64_REINTERPRET_F64: u8 = 0xBD;
    pub const OP_F32_REINTERPRET_I32: u8 = 0xBE;
    pub const OP_F64_REINTERPRET_I64: u8 = 0xBF;
}

#[cfg(feature = "llvm-jit")]
#[derive(Clone, Debug)]
struct BlockInfo {
    kind: BlockKind,
    start_block: LLVMBasicBlockRef,
    end_block: LLVMBasicBlockRef,
}

#[cfg(feature = "llvm-jit")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    Block,
    Loop,
    If,
}

pub struct WasmToLlvmTranslator {
    #[cfg(feature = "llvm-jit")]
    context: LLVMContextRef,
    #[cfg(feature = "llvm-jit")]
    builder: LLVMBuilderRef,
    #[cfg(feature = "llvm-jit")]
    locals: HashMap<u32, LLVMValueRef>,
    #[cfg(feature = "llvm-jit")]
    block_stack: Vec<BlockInfo>,
}

impl WasmToLlvmTranslator {
    #[cfg(feature = "llvm-jit")]
    pub fn new(context: LLVMContextRef) -> Result<Self> {
        unsafe {
            let builder = LLVMCreateBuilderInContext(context);
            if builder.is_null() {
                return Err(WasmError::Runtime(
                    "Failed to create LLVM builder".to_string(),
                ));
            }

            Ok(Self {
                context,
                builder,
                locals: HashMap::new(),
                block_stack: Vec::new(),
            })
        }
    }

    #[cfg(not(feature = "llvm-jit"))]
    pub fn new(_context: *mut std::ffi::c_void) -> Result<Self> {
        Err(WasmError::Runtime(
            "LLVM JIT not available: compile with --features llvm-jit".to_string(),
        ))
    }

    #[cfg(feature = "llvm-jit")]
    pub fn translate_function(
        &mut self,
        func: &Func,
        func_idx: u32,
        module: &Module,
    ) -> Result<(LLVMModuleRef, LLVMValueRef)> {
        unsafe {
            self.locals.clear();

            let func_type = module.func_type(func_idx).ok_or_else(|| {
                WasmError::Runtime(format!("Function type not found for {}", func_idx))
            })?;

            let llvm_func_type = self.create_function_type(func_type)?;
            let func_name = format!("wasm_func_{}", func_idx);
            let func_name_c = std::ffi::CString::new(func_name).unwrap();

            let module_name = format!("wasm_module_func_{}", func_idx);
            let module_name_c = std::ffi::CString::new(module_name).unwrap();

            let llvm_module =
                LLVMModuleCreateWithNameInContext(module_name_c.as_ptr(), self.context);
            let function = LLVMAddFunction(llvm_module, func_name_c.as_ptr(), llvm_func_type);

            let entry_block =
                LLVMAppendBasicBlockInContext(self.context, function, c"entry".as_ptr());
            LLVMPositionBuilderAtEnd(self.builder, entry_block);

            let param_count = LLVMCountParams(function);
            for i in 0..param_count {
                let param = LLVMGetParam(function, i);
                let param_type = LLVMTypeOf(param);
                let alloca = LLVMBuildAlloca(self.builder, param_type, c"param".as_ptr());
                LLVMBuildStore(self.builder, param, alloca);
                self.locals.insert(i, alloca);
            }

            let local_offset = param_count as u32;
            for (i, local) in func.locals.iter().enumerate() {
                for j in 0..local.count {
                    let local_idx = local_offset + i as u32 * local.count + j;
                    let llvm_type = self.wasm_type_to_llvm(local.type_)?;
                    let alloca = LLVMBuildAlloca(self.builder, llvm_type, c"local".as_ptr());

                    let zero = match local.type_ {
                        ValType::Num(NumType::I32) => {
                            LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0)
                        }
                        ValType::Num(NumType::I64) => {
                            LLVMConstInt(LLVMInt64TypeInContext(self.context), 0, 0)
                        }
                        ValType::Num(NumType::F32) => {
                            LLVMConstReal(LLVMFloatTypeInContext(self.context), 0.0)
                        }
                        ValType::Num(NumType::F64) => {
                            LLVMConstReal(LLVMDoubleTypeInContext(self.context), 0.0)
                        }
                        _ => continue,
                    };
                    LLVMBuildStore(self.builder, zero, alloca);
                    self.locals.insert(local_idx, alloca);
                }
            }

            self.translate_body(&func.body, function)?;

            Ok((llvm_module, function))
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn create_function_type(
        &self,
        func_type: &crate::runtime::FunctionType,
    ) -> Result<LLVMTypeRef> {
        let mut param_types: Vec<LLVMTypeRef> = Vec::new();
        for param in &func_type.params {
            param_types.push(self.wasm_type_to_llvm(*param)?);
        }

        let return_count = func_type.results.len() as u32;

        let return_type = if return_count == 0 {
            LLVMVoidTypeInContext(self.context)
        } else if return_count == 1 {
            self.wasm_type_to_llvm(func_type.results[0])?
        } else {
            let mut return_types: Vec<LLVMTypeRef> = Vec::new();
            for result in &func_type.results {
                return_types.push(self.wasm_type_to_llvm(*result)?);
            }
            LLVMStructTypeInContext(self.context, return_types.as_mut_ptr(), return_count, 0)
        };

        Ok(LLVMFunctionType(
            return_type,
            param_types.as_mut_ptr(),
            param_types.len() as u32,
            0,
        ))
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn wasm_type_to_llvm(&self, val_type: ValType) -> Result<LLVMTypeRef> {
        match val_type {
            ValType::Num(NumType::I32) => Ok(LLVMInt32TypeInContext(self.context)),
            ValType::Num(NumType::I64) => Ok(LLVMInt64TypeInContext(self.context)),
            ValType::Num(NumType::F32) => Ok(LLVMFloatTypeInContext(self.context)),
            ValType::Num(NumType::F64) => Ok(LLVMDoubleTypeInContext(self.context)),
            ValType::Ref(_) => Ok(LLVMInt8TypeInContext(self.context)),
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn translate_body(&mut self, bytecode: &[u8], _function: LLVMValueRef) -> Result<()> {
        let mut pc: usize = 0;
        let mut value_stack: Vec<LLVMValueRef> = Vec::new();

        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            match opcode {
                0x00 => {
                    LLVMBuildUnreachable(self.builder);
                    return Ok(());
                }
                0x01 => {
                    pc += 1;
                }
                0x20 => {
                    pc += 1;
                    let local_idx = self.read_uleb(bytecode, &mut pc)?;
                    if let Some(&alloca) = self.locals.get(&local_idx) {
                        let ptr_type = LLVMGetAllocatedType(alloca);
                        let value =
                            LLVMBuildLoad2(self.builder, ptr_type, alloca, c"local".as_ptr());
                        value_stack.push(value);
                    }
                }
                0x21 => {
                    pc += 1;
                    let local_idx = self.read_uleb(bytecode, &mut pc)?;
                    if let Some(value) = value_stack.pop()
                        && let Some(&alloca) = self.locals.get(&local_idx)
                    {
                        LLVMBuildStore(self.builder, value, alloca);
                    }
                }
                0x22 => {
                    pc += 1;
                    let local_idx = self.read_uleb(bytecode, &mut pc)?;
                    if let Some(&value) = value_stack.last()
                        && let Some(&alloca) = self.locals.get(&local_idx)
                    {
                        LLVMBuildStore(self.builder, value, alloca);
                    }
                }
                0x41 => {
                    pc += 1;
                    let val = self.read_uleb(bytecode, &mut pc)? as i32;
                    let const_val =
                        LLVMConstInt(LLVMInt32TypeInContext(self.context), val as u64, 1);
                    value_stack.push(const_val);
                }
                0x42 => {
                    pc += 1;
                    let val = self.read_uleb64(bytecode, &mut pc)? as i64;
                    let const_val =
                        LLVMConstInt(LLVMInt64TypeInContext(self.context), val as u64, 1);
                    value_stack.push(const_val);
                }
                0x43 => {
                    pc += 1;
                    let val = self.read_f32_bytes(bytecode, &mut pc)?;
                    let const_val = LLVMConstReal(LLVMFloatTypeInContext(self.context), val as f64);
                    value_stack.push(const_val);
                }
                0x44 => {
                    pc += 1;
                    let val = self.read_f64_bytes(bytecode, &mut pc)?;
                    let const_val = LLVMConstReal(LLVMDoubleTypeInContext(self.context), val);
                    value_stack.push(const_val);
                }
                0x6A => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildAdd(self.builder, a, b, c"add".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x6B => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildSub(self.builder, a, b, c"sub".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x6C => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildMul(self.builder, a, b, c"mul".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x6D => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildSDiv(self.builder, a, b, c"div_s".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x6E => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildUDiv(self.builder, a, b, c"div_u".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x71 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildAnd(self.builder, a, b, c"and".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x72 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildOr(self.builder, a, b, c"or".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x73 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildXor(self.builder, a, b, c"xor".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x74 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildShl(self.builder, a, b_trunc, c"shl".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x75 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildAShr(self.builder, a, b_trunc, c"shr_s".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x76 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildLShr(self.builder, a, b_trunc, c"shr_u".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x45 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let zero = LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0);
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntEQ,
                            a,
                            zero,
                            c"eqz".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"eqz_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x46 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntEQ,
                            a,
                            b,
                            c"eq".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"eq_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x47 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntNE,
                            a,
                            b,
                            c"ne".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"ne_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x48 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntSLT,
                            a,
                            b,
                            c"lt_s".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"lt_s_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x49 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntULT,
                            a,
                            b,
                            c"lt_u".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"lt_u_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4A => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntSGT,
                            a,
                            b,
                            c"gt_s".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"gt_s_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4B => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntUGT,
                            a,
                            b,
                            c"gt_u".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"gt_u_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4C => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntSLE,
                            a,
                            b,
                            c"le_s".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"le_s_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4D => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntULE,
                            a,
                            b,
                            c"le_u".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"le_u_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4E => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntSGE,
                            a,
                            b,
                            c"ge_s".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"ge_s_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x4F => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildICmp(
                            self.builder,
                            LLVMIntPredicate::LLVMIntUGE,
                            a,
                            b,
                            c"ge_u".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"ge_u_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x7C => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildAdd(self.builder, a, b, c"i64_add".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x7D => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildSub(self.builder, a, b, c"i64_sub".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x7E => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildMul(self.builder, a, b, c"i64_mul".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x7F => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildSDiv(self.builder, a, b, c"i64_div_s".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x80 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildUDiv(self.builder, a, b, c"i64_div_u".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x83 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildAnd(self.builder, a, b, c"i64_and".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x84 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildOr(self.builder, a, b, c"i64_or".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x85 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildXor(self.builder, a, b, c"i64_xor".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x86 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildShl(self.builder, a, b_trunc, c"i64_shl".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x87 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildAShr(self.builder, a, b_trunc, c"i64_shr_s".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x88 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let b_trunc = LLVMBuildTrunc(
                            self.builder,
                            b,
                            LLVMInt8TypeInContext(self.context),
                            c"shift".as_ptr(),
                        );
                        let result = LLVMBuildLShr(self.builder, a, b_trunc, c"i64_shr_u".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x8C => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFAdd(self.builder, a, b, c"f32_add".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x8D => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFSub(self.builder, a, b, c"f32_sub".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x8E => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFMul(self.builder, a, b, c"f32_mul".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x8F => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFDiv(self.builder, a, b, c"f32_div".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x90 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFRem(self.builder, a, b, c"f32_min".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x91 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFNeg(self.builder, a, c"f32_neg".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x92 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFAdd(self.builder, a, b, c"f64_add".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x93 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFSub(self.builder, a, b, c"f64_sub".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x94 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFMul(self.builder, a, b, c"f64_mul".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x95 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFDiv(self.builder, a, b, c"f64_div".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x96 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let result = LLVMBuildFRem(self.builder, a, b, c"f64_min".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x97 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFNeg(self.builder, a, c"f64_neg".as_ptr());
                        value_stack.push(result);
                    }
                }
                0x98 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPTrunc(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f64_demote".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x99 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPExt(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f32_promote".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA7 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOEQ,
                            a,
                            b,
                            c"f32_eq".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_eq_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA8 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealONE,
                            a,
                            b,
                            c"f32_ne".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_ne_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA9 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOLT,
                            a,
                            b,
                            c"f32_lt".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_lt_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAA => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOGT,
                            a,
                            b,
                            c"f32_gt".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_gt_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAB => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOLE,
                            a,
                            b,
                            c"f32_le".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_le_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAC => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOGE,
                            a,
                            b,
                            c"f32_ge".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f32_ge_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAD => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOEQ,
                            a,
                            b,
                            c"f64_eq".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_eq_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAE => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealONE,
                            a,
                            b,
                            c"f64_ne".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_ne_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xAF => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOLT,
                            a,
                            b,
                            c"f64_lt".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_lt_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xB0 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOGT,
                            a,
                            b,
                            c"f64_gt".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_gt_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xB1 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOLE,
                            a,
                            b,
                            c"f64_le".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_le_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xB2 => {
                    pc += 1;
                    if value_stack.len() >= 2 {
                        let b = value_stack.pop().unwrap();
                        let a = value_stack.pop().unwrap();
                        let cmp = LLVMBuildFCmp(
                            self.builder,
                            LLVMRealPredicate::LLVMRealOGE,
                            a,
                            b,
                            c"f64_ge".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            cmp,
                            LLVMInt32TypeInContext(self.context),
                            c"f64_ge_result".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xBC => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildBitCast(
                            self.builder,
                            a,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_reinterpret_f32".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xBD => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildBitCast(
                            self.builder,
                            a,
                            LLVMInt64TypeInContext(self.context),
                            c"i64_reinterpret_f64".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xBE => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildBitCast(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f32_reinterpret_i32".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xBF => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildBitCast(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f64_reinterpret_i64".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC0 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildSIToFP(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f32_convert_i32_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC1 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildUIToFP(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f32_convert_i32_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC2 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildSIToFP(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f32_convert_i64_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC3 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildUIToFP(
                            self.builder,
                            a,
                            LLVMFloatTypeInContext(self.context),
                            c"f32_convert_i64_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC4 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildSIToFP(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f64_convert_i32_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC5 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildUIToFP(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f64_convert_i32_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC6 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildSIToFP(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f64_convert_i64_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xC7 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildUIToFP(
                            self.builder,
                            a,
                            LLVMDoubleTypeInContext(self.context),
                            c"f64_convert_i64_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA2 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPToSI(
                            self.builder,
                            a,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_trunc_f32_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA3 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPToUI(
                            self.builder,
                            a,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_trunc_f32_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA4 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPToSI(
                            self.builder,
                            a,
                            LLVMInt64TypeInContext(self.context),
                            c"i64_trunc_f32_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA5 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPToUI(
                            self.builder,
                            a,
                            LLVMInt64TypeInContext(self.context),
                            c"i64_trunc_f32_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0xA6 => {
                    pc += 1;
                    if let Some(a) = value_stack.pop() {
                        let result = LLVMBuildFPToSI(
                            self.builder,
                            a,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_trunc_f64_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x1A => {
                    pc += 1;
                    value_stack.pop();
                }
                0x28 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt32TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt32TypeInContext(self.context),
                            ptr,
                            c"i32_load".as_ptr(),
                        );
                        value_stack.push(value);
                    }
                }
                0x29 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt64TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt64TypeInContext(self.context),
                            ptr,
                            c"i64_load".as_ptr(),
                        );
                        value_stack.push(value);
                    }
                }
                0x2A => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMFloatTypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMFloatTypeInContext(self.context),
                            ptr,
                            c"f32_load".as_ptr(),
                        );
                        value_stack.push(value);
                    }
                }
                0x2B => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMDoubleTypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMDoubleTypeInContext(self.context),
                            ptr,
                            c"f64_load".as_ptr(),
                        );
                        value_stack.push(value);
                    }
                }
                0x2C => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt8TypeInContext(self.context),
                            ptr,
                            c"i32_load8".as_ptr(),
                        );
                        let result = LLVMBuildSExt(
                            self.builder,
                            value,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_load8_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x2D => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt8TypeInContext(self.context),
                            ptr,
                            c"i32_load8".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            value,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_load8_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x2E => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt16TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt16TypeInContext(self.context),
                            ptr,
                            c"i32_load16".as_ptr(),
                        );
                        let result = LLVMBuildSExt(
                            self.builder,
                            value,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_load16_s".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x2F => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if let Some(addr) = value_stack.pop() {
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt16TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let value = LLVMBuildLoad2(
                            self.builder,
                            LLVMInt16TypeInContext(self.context),
                            ptr,
                            c"i32_load16".as_ptr(),
                        );
                        let result = LLVMBuildZExt(
                            self.builder,
                            value,
                            LLVMInt32TypeInContext(self.context),
                            c"i32_load16_u".as_ptr(),
                        );
                        value_stack.push(result);
                    }
                }
                0x36 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt32TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, value, ptr);
                    }
                }
                0x37 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt64TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, value, ptr);
                    }
                }
                0x38 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMFloatTypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, value, ptr);
                    }
                }
                0x39 => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMDoubleTypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, value, ptr);
                    }
                }
                0x3A => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt8TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let trunc = LLVMBuildTrunc(
                            self.builder,
                            value,
                            LLVMInt8TypeInContext(self.context),
                            c"i32_trunc8".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, trunc, ptr);
                    }
                }
                0x3B => {
                    pc += 1;
                    let _align = self.read_uleb(bytecode, &mut pc)?;
                    let offset = self.read_uleb(bytecode, &mut pc)? as i32;
                    if value_stack.len() >= 2 {
                        let value = value_stack.pop().unwrap();
                        let addr = value_stack.pop().unwrap();
                        let eff_addr = if offset != 0 {
                            let offset_val = LLVMConstInt(
                                LLVMInt32TypeInContext(self.context),
                                offset as u64,
                                1,
                            );
                            LLVMBuildAdd(self.builder, addr, offset_val, c"eff_addr".as_ptr())
                        } else {
                            addr
                        };
                        let ptr = LLVMBuildIntToPtr(
                            self.builder,
                            eff_addr,
                            LLVMPointerType(LLVMInt16TypeInContext(self.context), 0),
                            c"mem_ptr".as_ptr(),
                        );
                        let trunc = LLVMBuildTrunc(
                            self.builder,
                            value,
                            LLVMInt16TypeInContext(self.context),
                            c"i32_trunc16".as_ptr(),
                        );
                        LLVMBuildStore(self.builder, trunc, ptr);
                    }
                }
                0x02 => {
                    pc += 1;
                    let _block_type = self.read_uleb(bytecode, &mut pc)?;
                    let current_fn = LLVMGetBasicBlockParent(LLVMGetInsertBlock(self.builder));
                    let end_block = LLVMAppendBasicBlockInContext(
                        self.context,
                        current_fn,
                        c"block_end".as_ptr(),
                    );
                    self.block_stack.push(BlockInfo {
                        kind: BlockKind::Block,
                        start_block: LLVMGetInsertBlock(self.builder),
                        end_block,
                    });
                }
                0x03 => {
                    pc += 1;
                    let _block_type = self.read_uleb(bytecode, &mut pc)?;
                    let current_fn = LLVMGetBasicBlockParent(LLVMGetInsertBlock(self.builder));
                    let loop_header = LLVMAppendBasicBlockInContext(
                        self.context,
                        current_fn,
                        c"loop_header".as_ptr(),
                    );
                    let loop_exit = LLVMAppendBasicBlockInContext(
                        self.context,
                        current_fn,
                        c"loop_exit".as_ptr(),
                    );
                    LLVMBuildBr(self.builder, loop_header);
                    LLVMPositionBuilderAtEnd(self.builder, loop_header);
                    self.block_stack.push(BlockInfo {
                        kind: BlockKind::Loop,
                        start_block: loop_header,
                        end_block: loop_exit,
                    });
                }
                0x04 => {
                    pc += 1;
                    let _block_type = self.read_uleb(bytecode, &mut pc)?;
                    if let Some(cond) = value_stack.pop() {
                        let current_fn = LLVMGetBasicBlockParent(LLVMGetInsertBlock(self.builder));
                        let then_block = LLVMAppendBasicBlockInContext(
                            self.context,
                            current_fn,
                            c"if_then".as_ptr(),
                        );
                        let else_block = LLVMAppendBasicBlockInContext(
                            self.context,
                            current_fn,
                            c"if_else".as_ptr(),
                        );
                        let end_block = LLVMAppendBasicBlockInContext(
                            self.context,
                            current_fn,
                            c"if_end".as_ptr(),
                        );
                        let cond_i1 = LLVMBuildTruncOrBitCast(
                            self.builder,
                            cond,
                            LLVMInt1TypeInContext(self.context),
                            c"cond_i1".as_ptr(),
                        );
                        LLVMBuildCondBr(self.builder, cond_i1, then_block, else_block);
                        LLVMPositionBuilderAtEnd(self.builder, then_block);
                        self.block_stack.push(BlockInfo {
                            kind: BlockKind::If,
                            start_block: then_block,
                            end_block,
                        });
                    }
                }
                0x05 => {
                    pc += 1;
                    if let Some(block) = self.block_stack.last_mut()
                        && block.kind == BlockKind::If
                    {
                        LLVMBuildBr(self.builder, block.end_block);
                        let else_block = LLVMGetNextBasicBlock(LLVMGetInsertBlock(self.builder));
                        if !else_block.is_null() {
                            LLVMPositionBuilderAtEnd(self.builder, else_block);
                        }
                    }
                }
                0x0B => {
                    pc += 1;
                    if let Some(block) = self.block_stack.pop() {
                        LLVMBuildBr(self.builder, block.end_block);
                        LLVMPositionBuilderAtEnd(self.builder, block.end_block);
                    }
                }
                0x0C => {
                    pc += 1;
                    let label_idx = self.read_uleb(bytecode, &mut pc)? as usize;
                    if label_idx < self.block_stack.len() {
                        let target_idx = self.block_stack.len() - 1 - label_idx;
                        let block = &self.block_stack[target_idx];
                        match block.kind {
                            BlockKind::Loop => {
                                LLVMBuildBr(self.builder, block.start_block);
                            }
                            BlockKind::Block | BlockKind::If => {
                                LLVMBuildBr(self.builder, block.end_block);
                            }
                        }
                    }
                }
                0x0D => {
                    pc += 1;
                    let label_idx = self.read_uleb(bytecode, &mut pc)? as usize;
                    if let Some(cond) = value_stack.pop()
                        && label_idx < self.block_stack.len()
                    {
                        let target_idx = self.block_stack.len() - 1 - label_idx;
                        let block = &self.block_stack[target_idx];
                        let current_fn = LLVMGetBasicBlockParent(LLVMGetInsertBlock(self.builder));
                        let fallthrough = LLVMAppendBasicBlockInContext(
                            self.context,
                            current_fn,
                            c"br_if_fall".as_ptr(),
                        );
                        let cond_i1 = LLVMBuildTruncOrBitCast(
                            self.builder,
                            cond,
                            LLVMInt1TypeInContext(self.context),
                            c"cond_i1".as_ptr(),
                        );
                        match block.kind {
                            BlockKind::Loop => {
                                LLVMBuildCondBr(
                                    self.builder,
                                    cond_i1,
                                    block.start_block,
                                    fallthrough,
                                );
                            }
                            BlockKind::Block | BlockKind::If => {
                                LLVMBuildCondBr(
                                    self.builder,
                                    cond_i1,
                                    block.end_block,
                                    fallthrough,
                                );
                            }
                        }
                        LLVMPositionBuilderAtEnd(self.builder, fallthrough);
                    }
                }
                0x10 => {
                    pc += 1;
                    let func_idx = self.read_uleb(bytecode, &mut pc)?;
                    let current_block = LLVMGetInsertBlock(self.builder);
                    let current_fn = LLVMGetBasicBlockParent(current_block);
                    let module = LLVMGetGlobalParent(current_fn);
                    let callee_name = format!("wasm_func_{}", func_idx);
                    let callee_name_c = std::ffi::CString::new(callee_name.clone()).unwrap();
                    let callee = LLVMGetNamedFunction(module, callee_name_c.as_ptr());
                    if !callee.is_null() {
                        let param_count = LLVMCountParams(callee) as usize;
                        let mut args: Vec<LLVMValueRef> = Vec::new();
                        for _ in 0..param_count {
                            if let Some(arg) = value_stack.pop() {
                                args.push(arg);
                            }
                        }
                        args.reverse();
                        let result = LLVMBuildCall2(
                            self.builder,
                            LLVMGlobalGetValueType(callee),
                            callee,
                            args.as_mut_ptr(),
                            args.len() as u32,
                            c"call_result".as_ptr(),
                        );
                        let callee_type = LLVMGlobalGetValueType(callee);
                        if !LLVMGetReturnType(callee_type).is_null() {
                            let ret_type = LLVMGetReturnType(callee_type);
                            if ret_type != LLVMVoidTypeInContext(self.context) {
                                value_stack.push(result);
                            }
                        }
                    }
                }
                0x11 => {
                    pc += 1;
                    let _type_idx = self.read_uleb(bytecode, &mut pc)?;
                    let _table_idx = self.read_uleb(bytecode, &mut pc)?;
                    return Err(WasmError::Runtime(
                        "call_indirect not yet supported".to_string(),
                    ));
                }
                0x0F => {
                    if let Some(ret_val) = value_stack.last() {
                        LLVMBuildRet(self.builder, *ret_val);
                    } else {
                        LLVMBuildRetVoid(self.builder);
                    }
                    return Ok(());
                }
                _ => {
                    return Err(WasmError::Runtime(format!(
                        "Unsupported WASM opcode: 0x{:02X}",
                        opcode
                    )));
                }
            }
        }

        if value_stack.is_empty() {
            LLVMBuildRetVoid(self.builder);
        } else if let Some(ret_val) = value_stack.last() {
            LLVMBuildRet(self.builder, *ret_val);
        }

        Ok(())
    }

    #[cfg(feature = "llvm-jit")]
    fn read_uleb(&self, bytecode: &[u8], cursor: &mut usize) -> Result<u32> {
        let mut value = 0u32;
        let mut shift = 0u32;

        loop {
            let byte = *bytecode
                .get(*cursor)
                .ok_or_else(|| WasmError::Runtime("unexpected end of bytecode".to_string()))?;
            *cursor += 1;
            value |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Runtime("uleb128 overflow".to_string()));
            }
        }
    }

    #[cfg(feature = "llvm-jit")]
    fn read_uleb64(&self, bytecode: &[u8], cursor: &mut usize) -> Result<u64> {
        let mut value = 0u64;
        let mut shift = 0u32;

        loop {
            let byte = *bytecode
                .get(*cursor)
                .ok_or_else(|| WasmError::Runtime("unexpected end of bytecode".to_string()))?;
            *cursor += 1;
            value |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 70 {
                return Err(WasmError::Runtime("uleb128 overflow".to_string()));
            }
        }
    }

    #[cfg(feature = "llvm-jit")]
    fn read_f32_bytes(&self, bytecode: &[u8], cursor: &mut usize) -> Result<f32> {
        if *cursor + 4 > bytecode.len() {
            return Err(WasmError::Runtime(
                "unexpected end of bytecode reading f32".to_string(),
            ));
        }
        let bytes: [u8; 4] = [
            bytecode[*cursor],
            bytecode[*cursor + 1],
            bytecode[*cursor + 2],
            bytecode[*cursor + 3],
        ];
        *cursor += 4;
        Ok(f32::from_le_bytes(bytes))
    }

    #[cfg(feature = "llvm-jit")]
    fn read_f64_bytes(&self, bytecode: &[u8], cursor: &mut usize) -> Result<f64> {
        if *cursor + 8 > bytecode.len() {
            return Err(WasmError::Runtime(
                "unexpected end of bytecode reading f64".to_string(),
            ));
        }
        let bytes: [u8; 8] = [
            bytecode[*cursor],
            bytecode[*cursor + 1],
            bytecode[*cursor + 2],
            bytecode[*cursor + 3],
            bytecode[*cursor + 4],
            bytecode[*cursor + 5],
            bytecode[*cursor + 6],
            bytecode[*cursor + 7],
        ];
        *cursor += 8;
        Ok(f64::from_le_bytes(bytes))
    }
}

#[cfg(feature = "llvm-jit")]
impl Drop for WasmToLlvmTranslator {
    fn drop(&mut self) {
        unsafe {
            if !self.builder.is_null() {
                LLVMDisposeBuilder(self.builder);
            }
        }
    }
}

#[cfg(not(feature = "llvm-jit"))]
impl Drop for WasmToLlvmTranslator {
    fn drop(&mut self) {}
}

#[cfg(all(test, feature = "llvm-jit"))]
mod tests {
    use super::*;
    use crate::runtime::{Func, FunctionType, Local, Module, NumType, ValType};
    use llvm_sys::core::{LLVMContextCreate, LLVMContextDispose, LLVMDisposeModule};

    fn create_test_context() -> LLVMContextRef {
        unsafe { LLVMContextCreate() }
    }

    fn dispose_test_context(context: LLVMContextRef) {
        unsafe { LLVMContextDispose(context) };
    }

    fn create_simple_module() -> Module {
        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32), ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module
    }

    #[test]
    fn test_translator_creation() {
        let context = create_test_context();
        let result = WasmToLlvmTranslator::new(context);
        assert!(result.is_ok());
        drop(result);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_simple_add() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = create_simple_module();

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x6A, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_i32_const() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x2A, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_i64_arithmetic() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I64), ValType::Num(NumType::I64)],
            vec![ValType::Num(NumType::I64)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x7C, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_f32_arithmetic() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::F32), ValType::Num(NumType::F32)],
            vec![ValType::Num(NumType::F32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x8C, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_f64_arithmetic() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::F64), ValType::Num(NumType::F64)],
            vec![ValType::Num(NumType::F64)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x92, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_i32_comparison() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = create_simple_module();

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x20, 0x01, 0x46, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_local_ops() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x21, 0x00, 0x20, 0x00, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_void_function() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }

    #[test]
    fn test_translate_with_locals() {
        let context = create_test_context();
        let mut translator = WasmToLlvmTranslator::new(context).unwrap();
        let mut module = Module::new();

        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![Local {
                count: 1,
                type_: ValType::Num(NumType::I32),
            }],
            body: vec![0x41, 0x05, 0x21, 0x01, 0x20, 0x01, 0x0F],
        });

        let result = translator.translate_function(&module.funcs[0], 0, &module);
        assert!(result.is_ok());
        let (llvm_module, _) = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }
}
