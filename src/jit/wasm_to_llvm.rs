#![allow(unsafe_op_in_unsafe_fn)]

use crate::runtime::{Func, Module, NumType, Result, ValType, WasmError};
use std::collections::HashMap;
use std::ptr;

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
    #[cfg(feature = "llvm-jit")]
    current_function: LLVMValueRef,
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
                current_function: ptr::null_mut(),
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
    ) -> Result<LLVMModuleRef> {
        unsafe {
            self.locals.clear();
            self.block_stack.clear();

            let func_type = module.func_type(func_idx).ok_or_else(|| {
                WasmError::Runtime(format!("Function type not found for {}", func_idx))
            })?;

            let llvm_func_type = self.create_function_type(func_type)?;
            let func_name = format!("wasm_func_{}", func_idx);
            let func_name_c = std::ffi::CString::new(func_name)
                .map_err(|_| WasmError::Runtime("Function name contains NUL byte".to_string()))?;

            let module_name = format!("wasm_module_func_{}", func_idx);
            let module_name_c = std::ffi::CString::new(module_name)
                .map_err(|_| WasmError::Runtime("Module name contains NUL byte".to_string()))?;

            let llvm_module =
                LLVMModuleCreateWithNameInContext(module_name_c.as_ptr(), self.context);
            let function = LLVMAddFunction(llvm_module, func_name_c.as_ptr(), llvm_func_type);
            self.current_function = function;

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
            let mut next_local_idx = local_offset;
            for local in &func.locals {
                for _ in 0..local.count {
                    if matches!(local.type_, ValType::Ref(_)) {
                        return Err(WasmError::Runtime(
                            "LLVM JIT does not support reference-typed locals".to_string(),
                        ));
                    }
                    let local_idx = next_local_idx;
                    next_local_idx += 1;
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
                        ValType::Ref(_) => unreachable!(),
                    };
                    LLVMBuildStore(self.builder, zero, alloca);
                    self.locals.insert(local_idx, alloca);
                }
            }

            self.translate_body(&func.body, function, llvm_module, module)?;
            self.build_entry_wrapper(llvm_module, function, func_idx, func_type)?;

            Ok(llvm_module)
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
    unsafe fn get_or_declare_wasm_function(
        &mut self,
        llvm_module: LLVMModuleRef,
        func_idx: u32,
        wasm_module: &Module,
    ) -> Result<LLVMValueRef> {
        let func_name = format!("wasm_func_{}", func_idx);
        let func_name_c = std::ffi::CString::new(func_name)
            .map_err(|_| WasmError::Runtime("Function name contains NUL byte".to_string()))?;
        let existing = LLVMGetNamedFunction(llvm_module, func_name_c.as_ptr());
        if !existing.is_null() {
            return Ok(existing);
        }

        let func_type = wasm_module.func_type(func_idx).ok_or_else(|| {
            WasmError::Runtime(format!("Function type not found for {}", func_idx))
        })?;
        let llvm_func_type = self.create_function_type(func_type)?;
        Ok(LLVMAddFunction(
            llvm_module,
            func_name_c.as_ptr(),
            llvm_func_type,
        ))
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn get_or_declare_trap_status(&mut self, llvm_module: LLVMModuleRef) -> LLVMValueRef {
        let name_c = c"llvm_jit_has_trap";
        let existing = LLVMGetNamedFunction(llvm_module, name_c.as_ptr());
        if !existing.is_null() {
            return existing;
        }

        let func_type =
            LLVMFunctionType(LLVMInt32TypeInContext(self.context), ptr::null_mut(), 0, 0);
        LLVMAddFunction(llvm_module, name_c.as_ptr(), func_type)
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn get_or_declare_runtime_binary(
        &mut self,
        llvm_module: LLVMModuleRef,
        name: &str,
        lhs_type: LLVMTypeRef,
        rhs_type: LLVMTypeRef,
        ret_type: LLVMTypeRef,
    ) -> LLVMValueRef {
        let name_c =
            std::ffi::CString::new(name).expect("runtime helper name should not contain NUL");
        let existing = LLVMGetNamedFunction(llvm_module, name_c.as_ptr());
        if !existing.is_null() {
            return existing;
        }

        let mut param_types = [lhs_type, rhs_type];
        let func_type = LLVMFunctionType(ret_type, param_types.as_mut_ptr(), 2, 0);
        LLVMAddFunction(llvm_module, name_c.as_ptr(), func_type)
    }

    #[cfg(feature = "llvm-jit")]
    #[allow(clippy::too_many_arguments)]
    unsafe fn call_runtime_binary(
        &mut self,
        llvm_module: LLVMModuleRef,
        helper_name: &str,
        lhs_type: LLVMTypeRef,
        rhs_type: LLVMTypeRef,
        ret_type: LLVMTypeRef,
        lhs: LLVMValueRef,
        rhs: LLVMValueRef,
    ) -> LLVMValueRef {
        let helper = self.get_or_declare_runtime_binary(
            llvm_module,
            helper_name,
            lhs_type,
            rhs_type,
            ret_type,
        );
        let mut args = [lhs, rhs];
        LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(helper),
            helper,
            args.as_mut_ptr(),
            2,
            c"runtime_binary".as_ptr(),
        )
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn pack_value_as_i64(
        &self,
        value: LLVMValueRef,
        value_type: ValType,
    ) -> Result<LLVMValueRef> {
        let i64_type = LLVMInt64TypeInContext(self.context);
        match value_type {
            ValType::Num(NumType::I32) => Ok(LLVMBuildZExt(
                self.builder,
                value,
                i64_type,
                c"pack_i32".as_ptr(),
            )),
            ValType::Num(NumType::I64) => Ok(value),
            ValType::Num(NumType::F32) => {
                let bits = LLVMBuildBitCast(
                    self.builder,
                    value,
                    LLVMInt32TypeInContext(self.context),
                    c"f32_bits".as_ptr(),
                );
                Ok(LLVMBuildZExt(
                    self.builder,
                    bits,
                    i64_type,
                    c"pack_f32".as_ptr(),
                ))
            }
            ValType::Num(NumType::F64) => Ok(LLVMBuildBitCast(
                self.builder,
                value,
                i64_type,
                c"pack_f64".as_ptr(),
            )),
            ValType::Ref(_) => Err(WasmError::Runtime(
                "LLVM JIT does not support reference values yet".to_string(),
            )),
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn unpack_i64_value(
        &self,
        raw: LLVMValueRef,
        value_type: ValType,
    ) -> Result<LLVMValueRef> {
        match value_type {
            ValType::Num(NumType::I32) => Ok(LLVMBuildTrunc(
                self.builder,
                raw,
                LLVMInt32TypeInContext(self.context),
                c"unpack_i32".as_ptr(),
            )),
            ValType::Num(NumType::I64) => Ok(raw),
            ValType::Num(NumType::F32) => {
                let bits = LLVMBuildTrunc(
                    self.builder,
                    raw,
                    LLVMInt32TypeInContext(self.context),
                    c"f32_raw".as_ptr(),
                );
                Ok(LLVMBuildBitCast(
                    self.builder,
                    bits,
                    LLVMFloatTypeInContext(self.context),
                    c"unpack_f32".as_ptr(),
                ))
            }
            ValType::Num(NumType::F64) => Ok(LLVMBuildBitCast(
                self.builder,
                raw,
                LLVMDoubleTypeInContext(self.context),
                c"unpack_f64".as_ptr(),
            )),
            ValType::Ref(_) => Err(WasmError::Runtime(
                "LLVM JIT does not support reference values yet".to_string(),
            )),
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn build_default_return(&mut self) {
        let func_type = LLVMGlobalGetValueType(self.current_function);
        let return_type = LLVMGetReturnType(func_type);
        if return_type == LLVMVoidTypeInContext(self.context) {
            LLVMBuildRetVoid(self.builder);
        } else {
            LLVMBuildRet(self.builder, LLVMConstNull(return_type));
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn emit_trap_guard(&mut self, llvm_module: LLVMModuleRef) {
        let trap_status = self.get_or_declare_trap_status(llvm_module);
        let trap_code = LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(trap_status),
            trap_status,
            ptr::null_mut(),
            0,
            c"trap_status".as_ptr(),
        );
        let zero = LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0);
        let has_trap = LLVMBuildICmp(
            self.builder,
            LLVMIntPredicate::LLVMIntNE,
            trap_code,
            zero,
            c"has_trap".as_ptr(),
        );

        let continue_block = LLVMAppendBasicBlockInContext(
            self.context,
            self.current_function,
            c"trap_continue".as_ptr(),
        );
        let trap_block = LLVMAppendBasicBlockInContext(
            self.context,
            self.current_function,
            c"trap_return".as_ptr(),
        );
        LLVMBuildCondBr(self.builder, has_trap, trap_block, continue_block);

        LLVMPositionBuilderAtEnd(self.builder, trap_block);
        self.build_default_return();

        LLVMPositionBuilderAtEnd(self.builder, continue_block);
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn emit_meter_tick(&mut self, llvm_module: LLVMModuleRef, units: u64) {
        let helper_name = c"llvm_jit_meter_tick";
        let helper = match LLVMGetNamedFunction(llvm_module, helper_name.as_ptr()) {
            existing if !existing.is_null() => existing,
            _ => {
                let i64_type = LLVMInt64TypeInContext(self.context);
                let mut param_types = [i64_type];
                let func_type = LLVMFunctionType(
                    LLVMVoidTypeInContext(self.context),
                    param_types.as_mut_ptr(),
                    param_types.len() as u32,
                    0,
                );
                LLVMAddFunction(llvm_module, helper_name.as_ptr(), func_type)
            }
        };

        let i64_type = LLVMInt64TypeInContext(self.context);
        let mut args = [LLVMConstInt(i64_type, units, 0)];
        LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(helper),
            helper,
            args.as_mut_ptr(),
            args.len() as u32,
            c"".as_ptr(),
        );
        self.emit_trap_guard(llvm_module);
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn build_entry_wrapper(
        &mut self,
        llvm_module: LLVMModuleRef,
        function: LLVMValueRef,
        func_idx: u32,
        func_type: &crate::runtime::FunctionType,
    ) -> Result<()> {
        for value_type in func_type.params.iter().chain(func_type.results.iter()) {
            if matches!(value_type, ValType::Ref(_)) {
                return Err(WasmError::Runtime(
                    "LLVM JIT does not support reference-typed entry wrappers yet".to_string(),
                ));
            }
        }

        let wrapper_name = format!("wasm_entry_{}", func_idx);
        let wrapper_name_c = std::ffi::CString::new(wrapper_name)
            .map_err(|_| WasmError::Runtime("Wrapper name contains NUL byte".to_string()))?;
        let i64_type = LLVMInt64TypeInContext(self.context);
        let i64_ptr_type = LLVMPointerType(i64_type, 0);
        let mut wrapper_params = [i64_ptr_type, i64_ptr_type];
        let wrapper_type = LLVMFunctionType(
            LLVMVoidTypeInContext(self.context),
            wrapper_params.as_mut_ptr(),
            wrapper_params.len() as u32,
            0,
        );
        let wrapper = LLVMAddFunction(llvm_module, wrapper_name_c.as_ptr(), wrapper_type);
        let wrapper_entry = LLVMAppendBasicBlockInContext(self.context, wrapper, c"entry".as_ptr());
        let wrapper_continue =
            LLVMAppendBasicBlockInContext(self.context, wrapper, c"continue".as_ptr());
        let wrapper_suspend =
            LLVMAppendBasicBlockInContext(self.context, wrapper, c"suspend".as_ptr());
        LLVMPositionBuilderAtEnd(self.builder, wrapper_entry);

        let args_ptr = LLVMGetParam(wrapper, 0);
        let results_ptr = LLVMGetParam(wrapper, 1);

        let safepoint_helper_name = c"llvm_jit_safepoint_entry";
        let safepoint_helper =
            match LLVMGetNamedFunction(llvm_module, safepoint_helper_name.as_ptr()) {
                existing if !existing.is_null() => existing,
                _ => {
                    let mut param_types = [
                        LLVMInt32TypeInContext(self.context),
                        i64_ptr_type,
                        LLVMInt32TypeInContext(self.context),
                    ];
                    let helper_type = LLVMFunctionType(
                        LLVMInt32TypeInContext(self.context),
                        param_types.as_mut_ptr(),
                        param_types.len() as u32,
                        0,
                    );
                    LLVMAddFunction(llvm_module, safepoint_helper_name.as_ptr(), helper_type)
                }
            };
        let func_idx_value = LLVMConstInt(LLVMInt32TypeInContext(self.context), func_idx as u64, 0);
        let arg_count_value = LLVMConstInt(
            LLVMInt32TypeInContext(self.context),
            func_type.params.len() as u64,
            0,
        );
        let mut safepoint_args = [func_idx_value, args_ptr, arg_count_value];
        let safepoint_result = LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(safepoint_helper),
            safepoint_helper,
            safepoint_args.as_mut_ptr(),
            safepoint_args.len() as u32,
            c"entry_safepoint".as_ptr(),
        );
        let should_suspend = LLVMBuildICmp(
            self.builder,
            LLVMIntPredicate::LLVMIntNE,
            safepoint_result,
            LLVMConstInt(LLVMInt32TypeInContext(self.context), 0, 0),
            c"should_suspend".as_ptr(),
        );
        LLVMBuildCondBr(
            self.builder,
            should_suspend,
            wrapper_suspend,
            wrapper_continue,
        );

        LLVMPositionBuilderAtEnd(self.builder, wrapper_suspend);
        LLVMBuildRetVoid(self.builder);

        LLVMPositionBuilderAtEnd(self.builder, wrapper_continue);

        let mut call_args = Vec::with_capacity(func_type.params.len());
        for (idx, value_type) in func_type.params.iter().copied().enumerate() {
            let index = LLVMConstInt(i64_type, idx as u64, 0);
            let mut indices = [index];
            let slot_ptr = LLVMBuildGEP2(
                self.builder,
                i64_type,
                args_ptr,
                indices.as_mut_ptr(),
                1,
                c"arg_slot".as_ptr(),
            );
            let raw = LLVMBuildLoad2(self.builder, i64_type, slot_ptr, c"arg_raw".as_ptr());
            call_args.push(self.unpack_i64_value(raw, value_type)?);
        }

        let call_result = LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(function),
            function,
            call_args.as_mut_ptr(),
            call_args.len() as u32,
            if func_type.results.is_empty() {
                c"".as_ptr()
            } else {
                c"entry_call".as_ptr()
            },
        );

        if !func_type.results.is_empty() {
            for (idx, value_type) in func_type.results.iter().copied().enumerate() {
                let result_value = if func_type.results.len() == 1 {
                    call_result
                } else {
                    LLVMBuildExtractValue(
                        self.builder,
                        call_result,
                        idx as u32,
                        c"result_value".as_ptr(),
                    )
                };
                let packed = self.pack_value_as_i64(result_value, value_type)?;
                let index = LLVMConstInt(i64_type, idx as u64, 0);
                let mut indices = [index];
                let slot_ptr = LLVMBuildGEP2(
                    self.builder,
                    i64_type,
                    results_ptr,
                    indices.as_mut_ptr(),
                    1,
                    c"result_slot".as_ptr(),
                );
                LLVMBuildStore(self.builder, packed, slot_ptr);
            }
        }

        LLVMBuildRetVoid(self.builder);
        Ok(())
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn get_or_declare_runtime_load(
        &mut self,
        module: LLVMModuleRef,
        name: &str,
        ret_type: LLVMTypeRef,
    ) -> LLVMValueRef {
        let name_c =
            std::ffi::CString::new(name).expect("runtime helper name should not contain NUL");
        let existing = LLVMGetNamedFunction(module, name_c.as_ptr());
        if !existing.is_null() {
            return existing;
        }

        let i32_type = LLVMInt32TypeInContext(self.context);
        let func_type = LLVMFunctionType(ret_type, &i32_type as *const _ as *mut _, 1, 0);
        LLVMAddFunction(module, name_c.as_ptr(), func_type)
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn get_or_declare_runtime_store(
        &mut self,
        module: LLVMModuleRef,
        name: &str,
        val_type: LLVMTypeRef,
    ) -> LLVMValueRef {
        let name_c =
            std::ffi::CString::new(name).expect("runtime helper name should not contain NUL");
        let existing = LLVMGetNamedFunction(module, name_c.as_ptr());
        if !existing.is_null() {
            return existing;
        }

        let i32_type = LLVMInt32TypeInContext(self.context);
        let param_types = [i32_type, val_type];
        let func_type = LLVMFunctionType(
            LLVMVoidTypeInContext(self.context),
            param_types.as_ptr() as *mut _,
            2,
            0,
        );
        LLVMAddFunction(module, name_c.as_ptr(), func_type)
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn call_runtime_load(
        &mut self,
        module: LLVMModuleRef,
        helper_name: &str,
        ret_type: LLVMTypeRef,
        addr: LLVMValueRef,
    ) -> LLVMValueRef {
        let helper = self.get_or_declare_runtime_load(module, helper_name, ret_type);
        let mut args = [addr];
        LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(helper),
            helper,
            args.as_mut_ptr(),
            1,
            c"load_result".as_ptr(),
        )
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn call_runtime_store(
        &mut self,
        module: LLVMModuleRef,
        helper_name: &str,
        val_type: LLVMTypeRef,
        addr: LLVMValueRef,
        val: LLVMValueRef,
    ) {
        let helper = self.get_or_declare_runtime_store(module, helper_name, val_type);
        let mut args = [addr, val];
        LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(helper),
            helper,
            args.as_mut_ptr(),
            2,
            c"".as_ptr(),
        );
    }

    #[cfg(feature = "llvm-jit")]
    fn pop_binary_operands(
        &self,
        value_stack: &mut Vec<LLVMValueRef>,
    ) -> Option<(LLVMValueRef, LLVMValueRef)> {
        if value_stack.len() < 2 {
            return None;
        }

        let b = value_stack.pop()?;
        let a = value_stack.pop()?;
        Some((a, b))
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn read_effective_addr(
        &mut self,
        bytecode: &[u8],
        pc: &mut usize,
        addr: LLVMValueRef,
    ) -> Result<LLVMValueRef> {
        let _align = self.read_uleb(bytecode, pc)?;
        let offset = self.read_uleb(bytecode, pc)?;
        if offset == 0 {
            return Ok(addr);
        }

        let offset_val = LLVMConstInt(LLVMInt32TypeInContext(self.context), offset as u64, 0);
        Ok(LLVMBuildAdd(
            self.builder,
            addr,
            offset_val,
            c"eff_addr".as_ptr(),
        ))
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn translate_runtime_load_op(
        &mut self,
        bytecode: &[u8],
        pc: &mut usize,
        value_stack: &mut Vec<LLVMValueRef>,
        module: LLVMModuleRef,
        helper_name: &str,
        ret_type: LLVMTypeRef,
    ) -> Result<()> {
        *pc += 1;
        if let Some(addr) = value_stack.pop() {
            let eff_addr = self.read_effective_addr(bytecode, pc, addr)?;
            let value = self.call_runtime_load(module, helper_name, ret_type, eff_addr);
            value_stack.push(value);
            self.emit_trap_guard(module);
        }
        Ok(())
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn translate_runtime_store_op(
        &mut self,
        bytecode: &[u8],
        pc: &mut usize,
        value_stack: &mut Vec<LLVMValueRef>,
        module: LLVMModuleRef,
        helper_name: &str,
        val_type: LLVMTypeRef,
    ) -> Result<()> {
        *pc += 1;
        if let Some((addr, value)) = self.pop_binary_operands(value_stack) {
            let eff_addr = self.read_effective_addr(bytecode, pc, addr)?;
            self.call_runtime_store(module, helper_name, val_type, eff_addr, value);
            self.emit_trap_guard(module);
        }
        Ok(())
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn translate_import_call(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        llvm_module: LLVMModuleRef,
        func_idx: u32,
        func_type: &crate::runtime::FunctionType,
    ) -> Result<()> {
        for value_type in func_type.params.iter().chain(func_type.results.iter()) {
            if matches!(value_type, ValType::Ref(_)) {
                return Err(WasmError::Runtime(
                    "LLVM JIT does not support imported reference-typed calls yet".to_string(),
                ));
            }
        }

        let i64_type = LLVMInt64TypeInContext(self.context);
        let i64_ptr_type = LLVMPointerType(i64_type, 0);
        let helper_name = c"llvm_jit_call_import";
        let helper = match LLVMGetNamedFunction(llvm_module, helper_name.as_ptr()) {
            existing if !existing.is_null() => existing,
            _ => {
                let mut param_types = [
                    LLVMInt32TypeInContext(self.context),
                    i64_ptr_type,
                    LLVMInt32TypeInContext(self.context),
                    i64_ptr_type,
                    LLVMInt32TypeInContext(self.context),
                ];
                let func_type = LLVMFunctionType(
                    LLVMVoidTypeInContext(self.context),
                    param_types.as_mut_ptr(),
                    param_types.len() as u32,
                    0,
                );
                LLVMAddFunction(llvm_module, helper_name.as_ptr(), func_type)
            }
        };

        let mut packed_args = Vec::with_capacity(func_type.params.len());
        for value_type in func_type.params.iter().rev().copied() {
            let Some(arg) = value_stack.pop() else {
                return Ok(());
            };
            packed_args.push(self.pack_value_as_i64(arg, value_type)?);
        }
        packed_args.reverse();

        let arg_buffer = if packed_args.is_empty() {
            LLVMConstNull(i64_ptr_type)
        } else {
            let count = LLVMConstInt(i64_type, packed_args.len() as u64, 0);
            let buffer =
                LLVMBuildArrayAlloca(self.builder, i64_type, count, c"import_args".as_ptr());
            for (idx, arg) in packed_args.into_iter().enumerate() {
                let index = LLVMConstInt(i64_type, idx as u64, 0);
                let mut indices = [index];
                let slot_ptr = LLVMBuildGEP2(
                    self.builder,
                    i64_type,
                    buffer,
                    indices.as_mut_ptr(),
                    1,
                    c"import_arg_slot".as_ptr(),
                );
                LLVMBuildStore(self.builder, arg, slot_ptr);
            }
            buffer
        };

        let result_buffer = if func_type.results.is_empty() {
            LLVMConstNull(i64_ptr_type)
        } else {
            let count = LLVMConstInt(i64_type, func_type.results.len() as u64, 0);
            LLVMBuildArrayAlloca(self.builder, i64_type, count, c"import_results".as_ptr())
        };

        let func_idx_val = LLVMConstInt(LLVMInt32TypeInContext(self.context), func_idx as u64, 0);
        let arg_count = LLVMConstInt(
            LLVMInt32TypeInContext(self.context),
            func_type.params.len() as u64,
            0,
        );
        let result_count = LLVMConstInt(
            LLVMInt32TypeInContext(self.context),
            func_type.results.len() as u64,
            0,
        );
        let mut helper_args = [
            func_idx_val,
            arg_buffer,
            arg_count,
            result_buffer,
            result_count,
        ];
        LLVMBuildCall2(
            self.builder,
            LLVMGlobalGetValueType(helper),
            helper,
            helper_args.as_mut_ptr(),
            helper_args.len() as u32,
            c"".as_ptr(),
        );
        self.emit_trap_guard(llvm_module);

        if func_type.results.len() == 1 {
            let index = LLVMConstInt(i64_type, 0, 0);
            let mut indices = [index];
            let slot_ptr = LLVMBuildGEP2(
                self.builder,
                i64_type,
                result_buffer,
                indices.as_mut_ptr(),
                1,
                c"import_result_slot".as_ptr(),
            );
            let raw = LLVMBuildLoad2(
                self.builder,
                i64_type,
                slot_ptr,
                c"import_result_raw".as_ptr(),
            );
            value_stack.push(self.unpack_i64_value(raw, func_type.results[0])?);
        } else if func_type.results.len() > 1 {
            return Err(WasmError::Runtime(
                "LLVM JIT does not support multi-value imported calls yet".to_string(),
            ));
        }

        Ok(())
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_int_compare(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        predicate: LLVMIntPredicate,
        cmp_name: *const i8,
        result_name: *const i8,
    ) {
        if let Some((a, b)) = self.pop_binary_operands(value_stack) {
            let cmp = LLVMBuildICmp(self.builder, predicate, a, b, cmp_name);
            let result = LLVMBuildZExt(
                self.builder,
                cmp,
                LLVMInt32TypeInContext(self.context),
                result_name,
            );
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_float_compare(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        predicate: LLVMRealPredicate,
        cmp_name: *const i8,
        result_name: *const i8,
    ) {
        if let Some((a, b)) = self.pop_binary_operands(value_stack) {
            let cmp = LLVMBuildFCmp(self.builder, predicate, a, b, cmp_name);
            let result = LLVMBuildZExt(
                self.builder,
                cmp,
                LLVMInt32TypeInContext(self.context),
                result_name,
            );
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_binary_op<F>(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        name: *const i8,
        build: F,
    ) where
        F: FnOnce(LLVMBuilderRef, LLVMValueRef, LLVMValueRef, *const i8) -> LLVMValueRef,
    {
        if let Some((a, b)) = self.pop_binary_operands(value_stack) {
            let result = build(self.builder, a, b, name);
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_shift_op<F>(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        shift_type: LLVMTypeRef,
        shift_mask: u64,
        name: *const i8,
        build: F,
    ) where
        F: FnOnce(LLVMBuilderRef, LLVMValueRef, LLVMValueRef, *const i8) -> LLVMValueRef,
    {
        if let Some((a, b)) = self.pop_binary_operands(value_stack) {
            let mask = LLVMConstInt(shift_type, shift_mask, 0);
            let masked = LLVMBuildAnd(self.builder, b, mask, c"shift_masked".as_ptr());
            let shift =
                LLVMBuildTruncOrBitCast(self.builder, masked, shift_type, c"shift".as_ptr());
            let result = build(self.builder, a, shift, name);
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_unary_op<F>(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        name: *const i8,
        build: F,
    ) where
        F: FnOnce(LLVMBuilderRef, LLVMValueRef, *const i8) -> LLVMValueRef,
    {
        if let Some(a) = value_stack.pop() {
            let result = build(self.builder, a, name);
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn push_typed_unary_op<F>(
        &mut self,
        value_stack: &mut Vec<LLVMValueRef>,
        target_type: LLVMTypeRef,
        name: *const i8,
        build: F,
    ) where
        F: FnOnce(LLVMBuilderRef, LLVMValueRef, LLVMTypeRef, *const i8) -> LLVMValueRef,
    {
        if let Some(a) = value_stack.pop() {
            let result = build(self.builder, a, target_type, name);
            value_stack.push(result);
        }
    }

    #[cfg(feature = "llvm-jit")]
    unsafe fn translate_body(
        &mut self,
        bytecode: &[u8],
        function: LLVMValueRef,
        module: LLVMModuleRef,
        wasm_module: &Module,
    ) -> Result<()> {
        self.current_function = function;
        let mut pc: usize = 0;
        let mut value_stack: Vec<LLVMValueRef> = Vec::new();

        while pc < bytecode.len() {
            self.emit_meter_tick(module, 1);
            let opcode = bytecode[pc];
            match opcode {
                0x00 => {
                    let trap_helper =
                        match LLVMGetNamedFunction(module, c"llvm_jit_trap_unreachable".as_ptr()) {
                            existing if !existing.is_null() => existing,
                            _ => {
                                let func_type = LLVMFunctionType(
                                    LLVMVoidTypeInContext(self.context),
                                    ptr::null_mut(),
                                    0,
                                    0,
                                );
                                LLVMAddFunction(
                                    module,
                                    c"llvm_jit_trap_unreachable".as_ptr(),
                                    func_type,
                                )
                            }
                        };
                    LLVMBuildCall2(
                        self.builder,
                        LLVMGlobalGetValueType(trap_helper),
                        trap_helper,
                        ptr::null_mut(),
                        0,
                        c"".as_ptr(),
                    );
                    self.build_default_return();
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
                    let val = self.read_sleb32(bytecode, &mut pc)?;
                    let const_val =
                        LLVMConstInt(LLVMInt32TypeInContext(self.context), val as u64, 1);
                    value_stack.push(const_val);
                }
                0x42 => {
                    pc += 1;
                    let val = self.read_sleb64(bytecode, &mut pc)?;
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
                    self.push_binary_op(
                        &mut value_stack,
                        c"add".as_ptr(),
                        |builder, a, b, name| LLVMBuildAdd(builder, a, b, name),
                    );
                }
                0x6B => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"sub".as_ptr(),
                        |builder, a, b, name| LLVMBuildSub(builder, a, b, name),
                    );
                }
                0x6C => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"mul".as_ptr(),
                        |builder, a, b, name| LLVMBuildMul(builder, a, b, name),
                    );
                }
                0x6D => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i32_div_s",
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x6E => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i32_div_u",
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x6F => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i32_rem_s",
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x70 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i32_rem_u",
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            LLVMInt32TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x71 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"and".as_ptr(),
                        |builder, a, b, name| LLVMBuildAnd(builder, a, b, name),
                    );
                }
                0x72 => {
                    pc += 1;
                    self.push_binary_op(&mut value_stack, c"or".as_ptr(), |builder, a, b, name| {
                        LLVMBuildOr(builder, a, b, name)
                    });
                }
                0x73 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"xor".as_ptr(),
                        |builder, a, b, name| LLVMBuildXor(builder, a, b, name),
                    );
                }
                0x74 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt32TypeInContext(self.context),
                        31,
                        c"shl".as_ptr(),
                        |builder, a, b, name| LLVMBuildShl(builder, a, b, name),
                    );
                }
                0x75 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt32TypeInContext(self.context),
                        31,
                        c"shr_s".as_ptr(),
                        |builder, a, b, name| LLVMBuildAShr(builder, a, b, name),
                    );
                }
                0x76 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt32TypeInContext(self.context),
                        31,
                        c"shr_u".as_ptr(),
                        |builder, a, b, name| LLVMBuildLShr(builder, a, b, name),
                    );
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
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntEQ,
                        c"eq".as_ptr(),
                        c"eq_result".as_ptr(),
                    );
                }
                0x47 => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntNE,
                        c"ne".as_ptr(),
                        c"ne_result".as_ptr(),
                    );
                }
                0x48 => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntSLT,
                        c"lt_s".as_ptr(),
                        c"lt_s_result".as_ptr(),
                    );
                }
                0x49 => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntULT,
                        c"lt_u".as_ptr(),
                        c"lt_u_result".as_ptr(),
                    );
                }
                0x4A => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntSGT,
                        c"gt_s".as_ptr(),
                        c"gt_s_result".as_ptr(),
                    );
                }
                0x4B => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntUGT,
                        c"gt_u".as_ptr(),
                        c"gt_u_result".as_ptr(),
                    );
                }
                0x4C => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntSLE,
                        c"le_s".as_ptr(),
                        c"le_s_result".as_ptr(),
                    );
                }
                0x4D => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntULE,
                        c"le_u".as_ptr(),
                        c"le_u_result".as_ptr(),
                    );
                }
                0x4E => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntSGE,
                        c"ge_s".as_ptr(),
                        c"ge_s_result".as_ptr(),
                    );
                }
                0x4F => {
                    pc += 1;
                    self.push_int_compare(
                        &mut value_stack,
                        LLVMIntPredicate::LLVMIntUGE,
                        c"ge_u".as_ptr(),
                        c"ge_u_result".as_ptr(),
                    );
                }
                0x5B => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOEQ,
                        c"f32_eq".as_ptr(),
                        c"f32_eq_result".as_ptr(),
                    );
                }
                0x5C => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealONE,
                        c"f32_ne".as_ptr(),
                        c"f32_ne_result".as_ptr(),
                    );
                }
                0x5D => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOLT,
                        c"f32_lt".as_ptr(),
                        c"f32_lt_result".as_ptr(),
                    );
                }
                0x5E => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOGT,
                        c"f32_gt".as_ptr(),
                        c"f32_gt_result".as_ptr(),
                    );
                }
                0x5F => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOLE,
                        c"f32_le".as_ptr(),
                        c"f32_le_result".as_ptr(),
                    );
                }
                0x60 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOGE,
                        c"f32_ge".as_ptr(),
                        c"f32_ge_result".as_ptr(),
                    );
                }
                0x61 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOEQ,
                        c"f64_eq".as_ptr(),
                        c"f64_eq_result".as_ptr(),
                    );
                }
                0x62 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealONE,
                        c"f64_ne".as_ptr(),
                        c"f64_ne_result".as_ptr(),
                    );
                }
                0x63 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOLT,
                        c"f64_lt".as_ptr(),
                        c"f64_lt_result".as_ptr(),
                    );
                }
                0x64 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOGT,
                        c"f64_gt".as_ptr(),
                        c"f64_gt_result".as_ptr(),
                    );
                }
                0x65 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOLE,
                        c"f64_le".as_ptr(),
                        c"f64_le_result".as_ptr(),
                    );
                }
                0x66 => {
                    pc += 1;
                    self.push_float_compare(
                        &mut value_stack,
                        LLVMRealPredicate::LLVMRealOGE,
                        c"f64_ge".as_ptr(),
                        c"f64_ge_result".as_ptr(),
                    );
                }
                0x7C => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_add".as_ptr(),
                        |builder, a, b, name| LLVMBuildAdd(builder, a, b, name),
                    );
                }
                0x7D => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_sub".as_ptr(),
                        |builder, a, b, name| LLVMBuildSub(builder, a, b, name),
                    );
                }
                0x7E => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_mul".as_ptr(),
                        |builder, a, b, name| LLVMBuildMul(builder, a, b, name),
                    );
                }
                0x7F => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i64_div_s",
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x80 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i64_div_u",
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x81 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i64_rem_s",
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x82 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_i64_rem_u",
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            LLVMInt64TypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                        self.emit_trap_guard(module);
                    }
                }
                0x83 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_and".as_ptr(),
                        |builder, a, b, name| LLVMBuildAnd(builder, a, b, name),
                    );
                }
                0x84 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_or".as_ptr(),
                        |builder, a, b, name| LLVMBuildOr(builder, a, b, name),
                    );
                }
                0x85 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"i64_xor".as_ptr(),
                        |builder, a, b, name| LLVMBuildXor(builder, a, b, name),
                    );
                }
                0x86 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt64TypeInContext(self.context),
                        63,
                        c"i64_shl".as_ptr(),
                        |builder, a, b, name| LLVMBuildShl(builder, a, b, name),
                    );
                }
                0x87 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt64TypeInContext(self.context),
                        63,
                        c"i64_shr_s".as_ptr(),
                        |builder, a, b, name| LLVMBuildAShr(builder, a, b, name),
                    );
                }
                0x88 => {
                    pc += 1;
                    self.push_shift_op(
                        &mut value_stack,
                        LLVMInt64TypeInContext(self.context),
                        63,
                        c"i64_shr_u".as_ptr(),
                        |builder, a, b, name| LLVMBuildLShr(builder, a, b, name),
                    );
                }
                0x8C => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f32_add".as_ptr(),
                        |builder, a, b, name| LLVMBuildFAdd(builder, a, b, name),
                    );
                }
                0x8D => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f32_sub".as_ptr(),
                        |builder, a, b, name| LLVMBuildFSub(builder, a, b, name),
                    );
                }
                0x8E => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f32_mul".as_ptr(),
                        |builder, a, b, name| LLVMBuildFMul(builder, a, b, name),
                    );
                }
                0x8F => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f32_div".as_ptr(),
                        |builder, a, b, name| LLVMBuildFDiv(builder, a, b, name),
                    );
                }
                0x90 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_f32_min",
                            LLVMFloatTypeInContext(self.context),
                            LLVMFloatTypeInContext(self.context),
                            LLVMFloatTypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                    }
                }
                0x91 => {
                    pc += 1;
                    self.push_unary_op(
                        &mut value_stack,
                        c"f32_neg".as_ptr(),
                        |builder, a, name| LLVMBuildFNeg(builder, a, name),
                    );
                }
                0x92 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f64_add".as_ptr(),
                        |builder, a, b, name| LLVMBuildFAdd(builder, a, b, name),
                    );
                }
                0x93 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f64_sub".as_ptr(),
                        |builder, a, b, name| LLVMBuildFSub(builder, a, b, name),
                    );
                }
                0x94 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f64_mul".as_ptr(),
                        |builder, a, b, name| LLVMBuildFMul(builder, a, b, name),
                    );
                }
                0x95 => {
                    pc += 1;
                    self.push_binary_op(
                        &mut value_stack,
                        c"f64_div".as_ptr(),
                        |builder, a, b, name| LLVMBuildFDiv(builder, a, b, name),
                    );
                }
                0x96 => {
                    pc += 1;
                    if let Some((a, b)) = self.pop_binary_operands(&mut value_stack) {
                        let result = self.call_runtime_binary(
                            module,
                            "llvm_jit_f64_min",
                            LLVMDoubleTypeInContext(self.context),
                            LLVMDoubleTypeInContext(self.context),
                            LLVMDoubleTypeInContext(self.context),
                            a,
                            b,
                        );
                        value_stack.push(result);
                    }
                }
                0x97 => {
                    pc += 1;
                    self.push_unary_op(
                        &mut value_stack,
                        c"f64_neg".as_ptr(),
                        |builder, a, name| LLVMBuildFNeg(builder, a, name),
                    );
                }
                0x98 => {
                    pc += 1;
                    self.push_typed_unary_op(
                        &mut value_stack,
                        LLVMFloatTypeInContext(self.context),
                        c"f64_demote".as_ptr(),
                        |builder, a, target_type, name| {
                            LLVMBuildFPTrunc(builder, a, target_type, name)
                        },
                    );
                }
                0x99 => {
                    pc += 1;
                    self.push_typed_unary_op(
                        &mut value_stack,
                        LLVMDoubleTypeInContext(self.context),
                        c"f32_promote".as_ptr(),
                        |builder, a, target_type, name| {
                            LLVMBuildFPExt(builder, a, target_type, name)
                        },
                    );
                }
                0x1A => {
                    pc += 1;
                    value_stack.pop();
                }
                0x28 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_load",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x29 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x2A => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_f32_load",
                        LLVMFloatTypeInContext(self.context),
                    )?;
                }
                0x2B => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_f64_load",
                        LLVMDoubleTypeInContext(self.context),
                    )?;
                }
                0x2C => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_load8_s",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x2D => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_load8_u",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x2E => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_load16_s",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x2F => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_load16_u",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x30 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load8_s",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x31 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load8_u",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x32 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load16_s",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x33 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load16_u",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x34 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load32_s",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x35 => {
                    self.translate_runtime_load_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_load32_u",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x36 => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_store",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x37 => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_store",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x38 => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_f32_store",
                        LLVMFloatTypeInContext(self.context),
                    )?;
                }
                0x39 => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_f64_store",
                        LLVMDoubleTypeInContext(self.context),
                    )?;
                }
                0x3A => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_store8",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x3B => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i32_store16",
                        LLVMInt32TypeInContext(self.context),
                    )?;
                }
                0x3C => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_store8",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x3D => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_store16",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x3E => {
                    self.translate_runtime_store_op(
                        bytecode,
                        &mut pc,
                        &mut value_stack,
                        module,
                        "llvm_jit_i64_store32",
                        LLVMInt64TypeInContext(self.context),
                    )?;
                }
                0x3F => {
                    pc += 1;
                    let _memory_idx = self.read_uleb(bytecode, &mut pc)?;
                    let helper_name = c"llvm_jit_memory_size";
                    let helper = match LLVMGetNamedFunction(module, helper_name.as_ptr()) {
                        existing if !existing.is_null() => existing,
                        _ => {
                            let func_type = LLVMFunctionType(
                                LLVMInt32TypeInContext(self.context),
                                ptr::null_mut(),
                                0,
                                0,
                            );
                            LLVMAddFunction(module, helper_name.as_ptr(), func_type)
                        }
                    };
                    let result = LLVMBuildCall2(
                        self.builder,
                        LLVMGlobalGetValueType(helper),
                        helper,
                        ptr::null_mut(),
                        0,
                        c"memory_size".as_ptr(),
                    );
                    self.emit_trap_guard(module);
                    value_stack.push(result);
                }
                0x40 => {
                    pc += 1;
                    let _memory_idx = self.read_uleb(bytecode, &mut pc)?;
                    if let Some(delta) = value_stack.pop() {
                        let helper_name = c"llvm_jit_memory_grow";
                        let helper = match LLVMGetNamedFunction(module, helper_name.as_ptr()) {
                            existing if !existing.is_null() => existing,
                            _ => {
                                let mut param_types = [LLVMInt32TypeInContext(self.context)];
                                let func_type = LLVMFunctionType(
                                    LLVMInt32TypeInContext(self.context),
                                    param_types.as_mut_ptr(),
                                    param_types.len() as u32,
                                    0,
                                );
                                LLVMAddFunction(module, helper_name.as_ptr(), func_type)
                            }
                        };
                        let mut args = [delta];
                        let result = LLVMBuildCall2(
                            self.builder,
                            LLVMGlobalGetValueType(helper),
                            helper,
                            args.as_mut_ptr(),
                            args.len() as u32,
                            c"memory_grow".as_ptr(),
                        );
                        self.emit_trap_guard(module);
                        value_stack.push(result);
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
                    let import_func_count = wasm_module
                        .imports
                        .iter()
                        .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
                        .count() as u32;
                    let callee_type = wasm_module.func_type(func_idx).ok_or_else(|| {
                        WasmError::Runtime(format!("Function type not found for {}", func_idx))
                    })?;
                    if callee_type.results.len() > 1 {
                        return Err(WasmError::Runtime(
                            "LLVM JIT does not support multi-value calls yet".to_string(),
                        ));
                    }

                    if func_idx < import_func_count {
                        self.translate_import_call(
                            &mut value_stack,
                            module,
                            func_idx,
                            callee_type,
                        )?;
                    } else {
                        let callee =
                            self.get_or_declare_wasm_function(module, func_idx, wasm_module)?;
                        let param_count = LLVMCountParams(callee) as usize;
                        let mut args = Vec::with_capacity(param_count);
                        for _ in 0..param_count {
                            let Some(arg) = value_stack.pop() else {
                                return Ok(());
                            };
                            args.push(arg);
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
                        self.emit_trap_guard(module);
                        let ret_type = LLVMGetReturnType(LLVMGlobalGetValueType(callee));
                        if ret_type != LLVMVoidTypeInContext(self.context) {
                            value_stack.push(result);
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
    fn read_sleb32(&self, bytecode: &[u8], cursor: &mut usize) -> Result<i32> {
        let mut value = 0i32;
        let mut shift = 0u32;
        let mut byte;

        loop {
            byte = *bytecode
                .get(*cursor)
                .ok_or_else(|| WasmError::Runtime("unexpected end of bytecode".to_string()))?;
            *cursor += 1;
            value |= ((byte & 0x7F) as i32) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
            if shift >= 35 {
                return Err(WasmError::Runtime("sleb128 overflow".to_string()));
            }
        }

        if shift < 32 && (byte & 0x40) != 0 {
            value |= !0i32 << shift;
        }

        Ok(value)
    }

    #[cfg(feature = "llvm-jit")]
    fn read_sleb64(&self, bytecode: &[u8], cursor: &mut usize) -> Result<i64> {
        let mut value = 0i64;
        let mut shift = 0u32;
        let mut byte;

        loop {
            byte = *bytecode
                .get(*cursor)
                .ok_or_else(|| WasmError::Runtime("unexpected end of bytecode".to_string()))?;
            *cursor += 1;
            value |= ((byte & 0x7F) as i64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
            if shift >= 70 {
                return Err(WasmError::Runtime("sleb128 overflow".to_string()));
            }
        }

        if shift < 64 && (byte & 0x40) != 0 {
            value |= !0i64 << shift;
        }

        Ok(value)
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
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
        let llvm_module = result.unwrap();
        unsafe { LLVMDisposeModule(llvm_module) };
        drop(translator);
        dispose_test_context(context);
    }
}
