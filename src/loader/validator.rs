use crate::loader::BinaryReader;
use crate::runtime::{
    DataKind, ElemKind, Func, FunctionType, GlobalType, ImportKind, Module, NumType, RefType,
    Result, ValType, WasmError,
};
use std::collections::HashSet;

pub struct Validator;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Function,
    Block,
    Loop,
    If,
}

#[derive(Clone)]
struct ValidationFrame {
    kind: FrameKind,
    height: usize,
    params: Vec<ValType>,
    results: Vec<ValType>,
    label_types: Vec<ValType>,
    allow_else: bool,
    unreachable: bool,
}

impl Validator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, module: &Module) -> Result<()> {
        self.validate_types(module)?;
        self.validate_imports(module)?;
        self.validate_functions(module)?;
        self.validate_instruction_set(module)?;
        self.validate_tables(module)?;
        self.validate_memories(module)?;
        self.validate_globals(module)?;
        self.validate_data(module)?;
        self.validate_elems(module)?;
        self.validate_exports(module)?;
        self.validate_start(module)?;
        Ok(())
    }

    fn validate_types(&self, module: &Module) -> Result<()> {
        for (i, func_type) in module.types.iter().enumerate() {
            if func_type.params.len() > 16 {
                return Err(WasmError::Validation(format!(
                    "type {}: too many parameters",
                    i
                )));
            }
        }
        Ok(())
    }

    fn validate_functions(&self, module: &Module) -> Result<()> {
        for (i, func) in module.funcs.iter().enumerate() {
            if func.type_idx as usize >= module.types.len() {
                return Err(WasmError::Validation(format!(
                    "function {}: invalid type index",
                    i
                )));
            }
        }
        Ok(())
    }

    fn validate_imports(&self, module: &Module) -> Result<()> {
        for (i, import) in module.imports.iter().enumerate() {
            if let ImportKind::Func(type_idx) = import.kind
                && type_idx as usize >= module.types.len()
            {
                return Err(WasmError::Validation(format!(
                    "import {}: invalid function type index",
                    i
                )));
            }
        }

        Ok(())
    }

    fn validate_instruction_set(&self, module: &Module) -> Result<()> {
        for (i, func) in module.funcs.iter().enumerate() {
            let func_type = module.type_at(func.type_idx).ok_or_else(|| {
                WasmError::Validation(format!("function {}: invalid type index", i))
            })?;
            self.validate_function_body(module, i, func, func_type)?;
        }
        Ok(())
    }

    fn validate_function_body(
        &self,
        module: &Module,
        func_idx: usize,
        func: &Func,
        func_type: &FunctionType,
    ) -> Result<()> {
        let mut cursor = 0usize;
        let code = func.body.as_slice();
        let mut type_stack = Vec::new();
        let mut control_frames = vec![ValidationFrame {
            kind: FrameKind::Function,
            height: 0,
            params: Vec::new(),
            results: func_type.results.clone(),
            label_types: func_type.results.clone(),
            allow_else: false,
            unreachable: false,
        }];
        let local_types = self.local_types(func, func_type);

        while cursor < code.len() {
            let opcode = *code.get(cursor).ok_or_else(|| {
                WasmError::Validation(format!("function {}: unexpected end of body", func_idx))
            })?;
            cursor += 1;
            match opcode {
                0x00 => self.mark_unreachable(&mut type_stack, control_frames.last_mut().unwrap()),
                0x01 => {}
                0x02 | 0x03 => {
                    let (params, results) = self.read_block_signature(module, code, &mut cursor)?;
                    self.require_types(
                        func_idx,
                        &type_stack,
                        control_frames.last().unwrap(),
                        &params,
                    )?;
                    let height = type_stack.len().saturating_sub(params.len());
                    let label_types = if opcode == 0x03 {
                        params.clone()
                    } else {
                        results.clone()
                    };
                    control_frames.push(ValidationFrame {
                        kind: if opcode == 0x03 {
                            FrameKind::Loop
                        } else {
                            FrameKind::Block
                        },
                        height,
                        params,
                        results,
                        label_types,
                        allow_else: false,
                        unreachable: false,
                    });
                }
                0x04 => {
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    let (params, results) = self.read_block_signature(module, code, &mut cursor)?;
                    self.require_types(
                        func_idx,
                        &type_stack,
                        control_frames.last().unwrap(),
                        &params,
                    )?;
                    let height = type_stack.len().saturating_sub(params.len());
                    control_frames.push(ValidationFrame {
                        kind: FrameKind::If,
                        height,
                        params,
                        results: results.clone(),
                        label_types: results,
                        allow_else: true,
                        unreachable: false,
                    });
                }
                0x05 => {
                    let frame = control_frames.last_mut().ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} has else without matching if",
                            func_idx
                        ))
                    })?;
                    if frame.kind != FrameKind::If || !frame.allow_else {
                        return Err(WasmError::Validation(format!(
                            "function {} has invalid else opcode",
                            func_idx
                        )));
                    }
                    self.require_types(func_idx, &type_stack, frame, &frame.results)?;
                    type_stack.truncate(frame.height);
                    type_stack.extend(frame.params.iter().copied());
                    frame.allow_else = false;
                    frame.unreachable = false;
                }
                0x0B => {
                    if self.finish_frame(func_idx, &mut type_stack, &mut control_frames)? {
                        if cursor != code.len() {
                            return Err(WasmError::Validation(format!(
                                "function {} has trailing bytes after final end",
                                func_idx
                            )));
                        }
                        return Ok(());
                    }
                }
                0x0C => {
                    let depth = Self::read_uleb(code, &mut cursor)? as usize;
                    let label_types = self.label_types(func_idx, &control_frames, depth)?;
                    self.pop_types(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        &label_types,
                    )?;
                    self.mark_unreachable(&mut type_stack, control_frames.last_mut().unwrap());
                }
                0x0D => {
                    let depth = Self::read_uleb(code, &mut cursor)? as usize;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    let label_types = self.label_types(func_idx, &control_frames, depth)?;
                    self.require_types(
                        func_idx,
                        &type_stack,
                        control_frames.last().unwrap(),
                        &label_types,
                    )?;
                }
                0x0E => {
                    let count = Self::read_uleb(code, &mut cursor)? as usize;
                    let mut labels = Vec::with_capacity(count);
                    for _ in 0..count {
                        labels.push(Self::read_uleb(code, &mut cursor)? as usize);
                    }
                    let default = Self::read_uleb(code, &mut cursor)? as usize;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    let default_types = self.label_types(func_idx, &control_frames, default)?;
                    for depth in labels {
                        let label_types = self.label_types(func_idx, &control_frames, depth)?;
                        if label_types != default_types {
                            return Err(WasmError::Validation(format!(
                                "function {} br_table targets must share the same label type",
                                func_idx
                            )));
                        }
                    }
                    self.pop_types(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        &default_types,
                    )?;
                    self.mark_unreachable(&mut type_stack, control_frames.last_mut().unwrap());
                }
                0x11 => {
                    let type_idx = Self::read_uleb(code, &mut cursor)?;
                    let table_idx = Self::read_uleb(code, &mut cursor)?;
                    let target_type = module.type_at(type_idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid type index {}",
                            func_idx, type_idx
                        ))
                    })?;
                    let table_type = module.table_at(table_idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid table index {}",
                            func_idx, table_idx
                        ))
                    })?;
                    if table_type.elem_type != RefType::FuncRef {
                        return Err(WasmError::Validation(format!(
                            "function {} call_indirect requires a funcref table",
                            func_idx
                        )));
                    }
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    self.pop_types(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        &target_type.params,
                    )?;
                    type_stack.extend(target_type.results.iter().copied());
                }
                0x28..=0x3E => {
                    if module.memory_at(0).is_none() {
                        return Err(WasmError::Validation(format!(
                            "function {} uses memory instructions without a memory",
                            func_idx
                        )));
                    }
                    Self::skip_uleb(code, &mut cursor)?;
                    Self::skip_uleb(code, &mut cursor)?;
                    self.validate_memory_instruction(
                        func_idx,
                        opcode,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                    )?;
                }
                0x3F | 0x40 => {
                    if module.memory_at(0).is_none() {
                        return Err(WasmError::Validation(format!(
                            "function {} uses memory instructions without a memory",
                            func_idx
                        )));
                    }
                    let immediate = Self::read_byte(code, &mut cursor)?;
                    if immediate != 0 {
                        let name = if opcode == 0x3F {
                            "memory.size"
                        } else {
                            "memory.grow"
                        };
                        return Err(WasmError::Validation(format!(
                            "function {} has invalid {} immediate {}",
                            func_idx, name, immediate
                        )));
                    }
                    if opcode == 0x40 {
                        self.pop_type(
                            func_idx,
                            &mut type_stack,
                            control_frames.last().unwrap(),
                            ValType::Num(NumType::I32),
                        )?;
                    }
                    type_stack.push(ValType::Num(NumType::I32));
                }
                0xD0 => match Self::read_byte(code, &mut cursor)? {
                    0x70 => type_stack.push(ValType::Ref(RefType::FuncRef)),
                    0x6F => type_stack.push(ValType::Ref(RefType::ExternRef)),
                    value => {
                        return Err(WasmError::Validation(format!(
                            "function {} has invalid ref.null type {:02x}",
                            func_idx, value
                        )));
                    }
                },
                0x41 => {
                    Self::skip_sleb(code, &mut cursor)?;
                    type_stack.push(ValType::Num(NumType::I32));
                }
                0x42 => {
                    Self::skip_sleb(code, &mut cursor)?;
                    type_stack.push(ValType::Num(NumType::I64));
                }
                0x43 => {
                    Self::skip_bytes(code, &mut cursor, 4)?;
                    type_stack.push(ValType::Num(NumType::F32));
                }
                0x44 => {
                    Self::skip_bytes(code, &mut cursor, 8)?;
                    type_stack.push(ValType::Num(NumType::F64));
                }
                0x10 => {
                    let target = Self::read_uleb(code, &mut cursor)?;
                    let target_type = module.func_type(target).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} calls invalid function {}",
                            func_idx, target
                        ))
                    })?;
                    self.pop_types(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        &target_type.params,
                    )?;
                    type_stack.extend(target_type.results.iter().copied());
                }
                0x20 => {
                    let idx = Self::read_uleb(code, &mut cursor)? as usize;
                    let local_type = *local_types.get(idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid local index {}",
                            func_idx, idx
                        ))
                    })?;
                    type_stack.push(local_type);
                }
                0x21 | 0x22 => {
                    let idx = Self::read_uleb(code, &mut cursor)? as usize;
                    let local_type = *local_types.get(idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid local index {}",
                            func_idx, idx
                        ))
                    })?;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        local_type,
                    )?;
                    if opcode == 0x22 {
                        type_stack.push(local_type);
                    }
                }
                0x23 => {
                    let idx = Self::read_uleb(code, &mut cursor)?;
                    let global_type = module.global_at(idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid global index {}",
                            func_idx, idx
                        ))
                    })?;
                    type_stack.push(global_type.content_type);
                }
                0x24 => {
                    let idx = Self::read_uleb(code, &mut cursor)?;
                    let global_type = module.global_at(idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid global index {}",
                            func_idx, idx
                        ))
                    })?;
                    if !global_type.mutable {
                        return Err(WasmError::Validation(format!(
                            "function {} writes immutable global {}",
                            func_idx, idx
                        )));
                    }
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        global_type.content_type,
                    )?;
                }
                0x25 => {
                    let table_idx = Self::read_uleb(code, &mut cursor)?;
                    let table_type = module.table_at(table_idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid table index {}",
                            func_idx, table_idx
                        ))
                    })?;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    type_stack.push(ValType::Ref(table_type.elem_type));
                }
                0x26 => {
                    let table_idx = Self::read_uleb(code, &mut cursor)?;
                    let table_type = module.table_at(table_idx).ok_or_else(|| {
                        WasmError::Validation(format!(
                            "function {} uses invalid table index {}",
                            func_idx, table_idx
                        ))
                    })?;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Ref(table_type.elem_type),
                    )?;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                }
                0x0F => {
                    let result_types = control_frames
                        .first()
                        .map(|frame| frame.results.clone())
                        .unwrap_or_default();
                    self.pop_types(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        &result_types,
                    )?;
                    self.mark_unreachable(&mut type_stack, control_frames.last_mut().unwrap());
                }
                0x1A => {
                    self.pop_any(func_idx, &mut type_stack, control_frames.last().unwrap())?;
                }
                0x1B => {
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    let rhs =
                        self.pop_any(func_idx, &mut type_stack, control_frames.last().unwrap())?;
                    let lhs =
                        self.pop_any(func_idx, &mut type_stack, control_frames.last().unwrap())?;
                    if lhs != rhs {
                        return Err(WasmError::Validation(format!(
                            "function {} select operands must have matching types",
                            func_idx
                        )));
                    }
                    type_stack.push(lhs);
                }
                0x45 => self.validate_unary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::I32),
                    ValType::Num(NumType::I32),
                )?,
                0x46..=0x4F => self.validate_binary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::I32),
                    ValType::Num(NumType::I32),
                )?,
                0x6A..=0x76 => self.validate_binary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::I32),
                    ValType::Num(NumType::I32),
                )?,
                0x79..=0x7B => self.validate_unary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::I64),
                    ValType::Num(NumType::I64),
                )?,
                0x7C..=0x85 => self.validate_binary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::I64),
                    ValType::Num(NumType::I64),
                )?,
                0x86..=0x88 => {
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I32),
                    )?;
                    self.pop_type(
                        func_idx,
                        &mut type_stack,
                        control_frames.last().unwrap(),
                        ValType::Num(NumType::I64),
                    )?;
                    type_stack.push(ValType::Num(NumType::I64));
                }
                0x92 => self.validate_binary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::F32),
                    ValType::Num(NumType::F32),
                )?,
                0xA6..=0xA9 => self.validate_binary_numeric(
                    func_idx,
                    &mut type_stack,
                    control_frames.last().unwrap(),
                    ValType::Num(NumType::F64),
                    ValType::Num(NumType::F64),
                )?,
                0xD1 => {
                    if control_frames.last().unwrap().unreachable {
                        type_stack.push(ValType::Num(NumType::I32));
                        continue;
                    }
                    let value =
                        self.pop_any(func_idx, &mut type_stack, control_frames.last().unwrap())?;
                    if !value.is_reference() {
                        return Err(WasmError::Validation(format!(
                            "function {} ref.is_null requires a reference operand",
                            func_idx
                        )));
                    }
                    type_stack.push(ValType::Num(NumType::I32));
                }
                0xD2 => {
                    let target = Self::read_uleb(code, &mut cursor)?;
                    if module.func_type(target).is_none() {
                        return Err(WasmError::Validation(format!(
                            "function {} references invalid function {}",
                            func_idx, target
                        )));
                    }
                    type_stack.push(ValType::Ref(RefType::FuncRef));
                }
                0xFC => {
                    return Err(WasmError::Validation(format!(
                        "function {} uses unsupported 0xfc-prefixed instructions",
                        func_idx
                    )));
                }
                _ => {
                    return Err(WasmError::Validation(format!(
                        "function {} uses unsupported opcode {:02x}",
                        func_idx, opcode
                    )));
                }
            }
        }

        Err(WasmError::Validation(format!(
            "function {} missing terminating end opcode",
            func_idx
        )))
    }

    fn local_types(&self, func: &Func, func_type: &FunctionType) -> Vec<ValType> {
        let mut locals = func_type.params.clone();
        for local in &func.locals {
            for _ in 0..local.count {
                locals.push(local.type_);
            }
        }
        locals
    }

    fn read_block_signature(
        &self,
        module: &Module,
        code: &[u8],
        cursor: &mut usize,
    ) -> Result<(Vec<ValType>, Vec<ValType>)> {
        let marker = Self::read_byte(code, cursor)?;
        match marker {
            0x40 => Ok((Vec::new(), Vec::new())),
            0x7F => Ok((Vec::new(), vec![ValType::Num(NumType::I32)])),
            0x7E => Ok((Vec::new(), vec![ValType::Num(NumType::I64)])),
            0x7D => Ok((Vec::new(), vec![ValType::Num(NumType::F32)])),
            0x7C => Ok((Vec::new(), vec![ValType::Num(NumType::F64)])),
            0x70 => Ok((Vec::new(), vec![ValType::Ref(RefType::FuncRef)])),
            0x6F => Ok((Vec::new(), vec![ValType::Ref(RefType::ExternRef)])),
            byte => {
                let type_idx = Self::read_signed_leb_continuation(code, cursor, byte)?;
                if type_idx < 0 {
                    return Err(WasmError::Validation(format!(
                        "invalid block type index {}",
                        type_idx
                    )));
                }
                let type_ = module
                    .type_at(type_idx as u32)
                    .ok_or_else(|| WasmError::Validation(format!("type {} not found", type_idx)))?;
                Ok((type_.params.clone(), type_.results.clone()))
            }
        }
    }

    fn finish_frame(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        control_frames: &mut Vec<ValidationFrame>,
    ) -> Result<bool> {
        let frame = control_frames.pop().ok_or_else(|| {
            WasmError::Validation(format!("function {} has unmatched end opcode", func_idx))
        })?;
        self.require_types(func_idx, type_stack, &frame, &frame.results)?;
        type_stack.truncate(frame.height);
        type_stack.extend(frame.results.iter().copied());
        Ok(control_frames.is_empty())
    }

    fn require_types(
        &self,
        func_idx: usize,
        type_stack: &[ValType],
        frame: &ValidationFrame,
        expected: &[ValType],
    ) -> Result<()> {
        if frame.unreachable {
            return Ok(());
        }
        if type_stack.len() < frame.height + expected.len() {
            return Err(WasmError::Validation(format!(
                "function {} stack underflow",
                func_idx
            )));
        }
        let actual = &type_stack[type_stack.len() - expected.len()..];
        if actual != expected {
            return Err(WasmError::Validation(format!(
                "function {} stack type mismatch",
                func_idx
            )));
        }
        Ok(())
    }

    fn pop_types(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
        expected: &[ValType],
    ) -> Result<()> {
        self.require_types(func_idx, type_stack, frame, expected)?;
        if !frame.unreachable {
            type_stack.truncate(type_stack.len() - expected.len());
        }
        Ok(())
    }

    fn pop_type(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
        expected: ValType,
    ) -> Result<()> {
        self.pop_types(func_idx, type_stack, frame, &[expected])
    }

    fn pop_any(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
    ) -> Result<ValType> {
        if frame.unreachable {
            return Ok(ValType::Num(NumType::I32));
        }
        if type_stack.len() <= frame.height {
            return Err(WasmError::Validation(format!(
                "function {} stack underflow",
                func_idx
            )));
        }
        type_stack
            .pop()
            .ok_or_else(|| WasmError::Validation(format!("function {} stack underflow", func_idx)))
    }

    fn validate_unary_numeric(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
        operand: ValType,
        result: ValType,
    ) -> Result<()> {
        self.pop_type(func_idx, type_stack, frame, operand)?;
        type_stack.push(result);
        Ok(())
    }

    fn validate_binary_numeric(
        &self,
        func_idx: usize,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
        operand: ValType,
        result: ValType,
    ) -> Result<()> {
        self.pop_type(func_idx, type_stack, frame, operand)?;
        self.pop_type(func_idx, type_stack, frame, operand)?;
        type_stack.push(result);
        Ok(())
    }

    fn validate_memory_instruction(
        &self,
        func_idx: usize,
        opcode: u8,
        type_stack: &mut Vec<ValType>,
        frame: &ValidationFrame,
    ) -> Result<()> {
        use NumType::{F32, F64, I32, I64};

        match opcode {
            0x28..=0x2F => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
                type_stack.push(ValType::Num(I32));
            }
            0x30..=0x35 => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
                type_stack.push(ValType::Num(I64));
            }
            0x36 => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            0x37 => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I64))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            0x38 => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(F32))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            0x39 => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(F64))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            0x3A | 0x3B => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            0x3C..=0x3E => {
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I64))?;
                self.pop_type(func_idx, type_stack, frame, ValType::Num(I32))?;
            }
            _ => {}
        }
        Ok(())
    }

    fn label_types(
        &self,
        func_idx: usize,
        control_frames: &[ValidationFrame],
        depth: usize,
    ) -> Result<Vec<ValType>> {
        control_frames
            .get(control_frames.len().checked_sub(depth + 1).ok_or_else(|| {
                WasmError::Validation(format!(
                    "function {} uses invalid branch depth {}",
                    func_idx, depth
                ))
            })?)
            .map(|frame| frame.label_types.clone())
            .ok_or_else(|| {
                WasmError::Validation(format!(
                    "function {} uses invalid branch depth {}",
                    func_idx, depth
                ))
            })
    }

    fn mark_unreachable(&self, type_stack: &mut Vec<ValType>, frame: &mut ValidationFrame) {
        type_stack.truncate(frame.height);
        frame.unreachable = true;
    }

    fn validate_tables(&self, module: &Module) -> Result<()> {
        for (i, table) in module.tables.iter().enumerate() {
            if table.limits.min() > 0x10000000 {
                return Err(WasmError::Validation(format!(
                    "table {}: minimum size too large",
                    i
                )));
            }
            if let Some(max) = table.limits.max() {
                if max > 0x10000000 {
                    return Err(WasmError::Validation(format!(
                        "table {}: maximum size too large",
                        i
                    )));
                }
                if max < table.limits.min() {
                    return Err(WasmError::Validation(format!(
                        "table {}: maximum less than minimum",
                        i
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_memories(&self, module: &Module) -> Result<()> {
        let total_memories = module.memories.len()
            + module
                .imports
                .iter()
                .filter(|import| matches!(import.kind, ImportKind::Memory(_)))
                .count();
        if total_memories > 1 {
            return Err(WasmError::Validation(
                "multi-memory modules are not supported".to_string(),
            ));
        }

        for (i, memory) in module.memories.iter().enumerate() {
            if memory.limits.min() > 65536 {
                return Err(WasmError::Validation(format!(
                    "memory {}: minimum size too large",
                    i
                )));
            }
            if let Some(max) = memory.limits.max() {
                if max > 65536 {
                    return Err(WasmError::Validation(format!(
                        "memory {}: maximum size too large",
                        i
                    )));
                }
                if max < memory.limits.min() {
                    return Err(WasmError::Validation(format!(
                        "memory {}: maximum less than minimum",
                        i
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_globals(&self, module: &Module) -> Result<()> {
        let imported_globals = self.imported_globals(module);

        for (i, global) in module.globals.iter().enumerate() {
            if !matches!(global.content_type, ValType::Num(_))
                && !matches!(global.content_type, ValType::Ref(_))
            {
                return Err(WasmError::Validation(format!("global {}: invalid type", i)));
            }

            let init = module.global_inits.get(i).ok_or_else(|| {
                WasmError::Validation(format!("global {}: missing init expression", i))
            })?;
            let value_type = self.validate_const_expr(module, init, imported_globals.as_slice())?;
            if value_type != global.content_type {
                return Err(WasmError::Validation(format!(
                    "global {}: init expression type mismatch",
                    i
                )));
            }
        }
        Ok(())
    }

    fn validate_data(&self, module: &Module) -> Result<()> {
        let imported_globals = self.imported_globals(module);
        let memory_count = module.memories.len()
            + module
                .imports
                .iter()
                .filter(|import| matches!(import.kind, ImportKind::Memory(_)))
                .count();

        for (i, segment) in module.data.iter().enumerate() {
            if let DataKind::Active { memory_idx, offset } = &segment.kind {
                if *memory_idx as usize >= memory_count {
                    return Err(WasmError::Validation(format!(
                        "data segment {}: invalid memory index",
                        i
                    )));
                }
                let value_type =
                    self.validate_const_expr(module, offset, imported_globals.as_slice())?;
                if value_type != ValType::Num(NumType::I32) {
                    return Err(WasmError::Validation(format!(
                        "data segment {}: offset expression must be i32",
                        i
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_elems(&self, module: &Module) -> Result<()> {
        let imported_globals = self.imported_globals(module);
        let table_count = module.tables.len()
            + module
                .imports
                .iter()
                .filter(|import| matches!(import.kind, ImportKind::Table(_)))
                .count();

        for (i, segment) in module.elems.iter().enumerate() {
            if let ElemKind::Active { table_idx, offset } = &segment.kind {
                if *table_idx as usize >= table_count {
                    return Err(WasmError::Validation(format!(
                        "element segment {}: invalid table index",
                        i
                    )));
                }
                let value_type =
                    self.validate_const_expr(module, offset, imported_globals.as_slice())?;
                if value_type != ValType::Num(NumType::I32) {
                    return Err(WasmError::Validation(format!(
                        "element segment {}: offset expression must be i32",
                        i
                    )));
                }
            }

            for expr in &segment.init {
                let value_type =
                    self.validate_const_expr(module, expr, imported_globals.as_slice())?;
                if value_type != ValType::Ref(segment.type_) {
                    return Err(WasmError::Validation(format!(
                        "element segment {}: init expression type mismatch",
                        i
                    )));
                }
            }
        }

        Ok(())
    }

    fn validate_exports(&self, module: &Module) -> Result<()> {
        let mut seen_names = HashSet::new();
        let table_count = module.tables.len()
            + module
                .imports
                .iter()
                .filter(|i| matches!(i.kind, ImportKind::Table(_)))
                .count();
        let memory_count = module.memories.len()
            + module
                .imports
                .iter()
                .filter(|i| matches!(i.kind, ImportKind::Memory(_)))
                .count();
        let global_count = module.globals.len()
            + module
                .imports
                .iter()
                .filter(|i| matches!(i.kind, ImportKind::Global(_)))
                .count();

        for (i, export) in module.exports.iter().enumerate() {
            if !seen_names.insert(export.name.as_str()) {
                return Err(WasmError::Validation(format!(
                    "duplicate export name: {}",
                    export.name
                )));
            }
            match &export.kind {
                crate::runtime::ExportKind::Func(idx) => {
                    if *idx as usize
                        >= module.funcs.len()
                            + module
                                .imports
                                .iter()
                                .filter(|i| matches!(i.kind, crate::runtime::ImportKind::Func(_)))
                                .count()
                    {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid function index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Table(idx) => {
                    if *idx as usize >= table_count {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid table index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Memory(idx) => {
                    if *idx as usize >= memory_count {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid memory index",
                            i
                        )));
                    }
                }
                crate::runtime::ExportKind::Global(idx) => {
                    if *idx as usize >= global_count {
                        return Err(WasmError::Validation(format!(
                            "export {}: invalid global index",
                            i
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn imported_globals(&self, module: &Module) -> Vec<GlobalType> {
        module
            .imports
            .iter()
            .filter_map(|import| match &import.kind {
                ImportKind::Global(global_type) => Some(global_type.clone()),
                _ => None,
            })
            .collect()
    }

    fn validate_const_expr(
        &self,
        module: &Module,
        expr: &[u8],
        imported_globals: &[GlobalType],
    ) -> Result<ValType> {
        let mut reader = BinaryReader::from_slice(expr);
        let opcode = reader.read_u8().map_err(WasmError::from)?;

        let value_type = match opcode {
            0x23 => {
                let idx = reader.read_uleb128().map_err(WasmError::from)? as usize;
                let global = imported_globals.get(idx).ok_or_else(|| {
                    WasmError::Validation(format!(
                        "constant expression references invalid global {}",
                        idx
                    ))
                })?;
                if global.mutable {
                    return Err(WasmError::Validation(format!(
                        "constant expression references mutable global {}",
                        idx
                    )));
                }
                global.content_type
            }
            0x41 => {
                let _ = reader.read_sleb128().map_err(WasmError::from)?;
                ValType::Num(NumType::I32)
            }
            0x42 => {
                let _ = reader.read_sleb128_i64().map_err(WasmError::from)?;
                ValType::Num(NumType::I64)
            }
            0x43 => {
                let _ = reader.read_f32().map_err(WasmError::from)?;
                ValType::Num(NumType::F32)
            }
            0x44 => {
                let _ = reader.read_f64().map_err(WasmError::from)?;
                ValType::Num(NumType::F64)
            }
            0xD0 => match reader.read_u8().map_err(WasmError::from)? {
                0x70 => ValType::Ref(crate::runtime::RefType::FuncRef),
                0x6F => ValType::Ref(crate::runtime::RefType::ExternRef),
                value => {
                    return Err(WasmError::Validation(format!(
                        "invalid ref.null type: {:02x}",
                        value
                    )));
                }
            },
            0xD2 => {
                let idx = reader.read_uleb128().map_err(WasmError::from)?;
                if idx >= module.func_count() {
                    return Err(WasmError::Validation(format!(
                        "constant expression references invalid function {}",
                        idx
                    )));
                }
                ValType::Ref(crate::runtime::RefType::FuncRef)
            }
            value => {
                return Err(WasmError::Validation(format!(
                    "unsupported constant expression opcode: {:02x}",
                    value
                )));
            }
        };

        let end = reader.read_u8().map_err(WasmError::from)?;
        if end != 0x0B {
            return Err(WasmError::Validation(
                "constant expression missing end opcode".to_string(),
            ));
        }
        if reader.remaining() != 0 {
            return Err(WasmError::Validation(
                "constant expression has trailing bytes".to_string(),
            ));
        }

        Ok(value_type)
    }

    fn skip_uleb(code: &[u8], cursor: &mut usize) -> Result<()> {
        let _ = Self::read_uleb(code, cursor)?;
        Ok(())
    }

    fn read_byte(code: &[u8], cursor: &mut usize) -> Result<u8> {
        let byte = *code
            .get(*cursor)
            .ok_or_else(|| WasmError::Validation("unexpected end of immediate".to_string()))?;
        *cursor += 1;
        Ok(byte)
    }

    fn read_uleb(code: &[u8], cursor: &mut usize) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0u32;
        loop {
            let byte = *code.get(*cursor).ok_or_else(|| {
                WasmError::Validation("unexpected end of uleb immediate".to_string())
            })?;
            *cursor += 1;
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Validation("uleb128 overflow".to_string()));
            }
        }
    }

    fn skip_sleb(code: &[u8], cursor: &mut usize) -> Result<()> {
        let first = *code
            .get(*cursor)
            .ok_or_else(|| WasmError::Validation("unexpected end of sleb immediate".to_string()))?;
        *cursor += 1;
        Self::skip_sleb_tail(code, cursor, first)
    }

    fn skip_sleb_tail(code: &[u8], cursor: &mut usize, mut byte: u8) -> Result<()> {
        while byte & 0x80 != 0 {
            byte = *code.get(*cursor).ok_or_else(|| {
                WasmError::Validation("unexpected end of sleb immediate".to_string())
            })?;
            *cursor += 1;
        }
        Ok(())
    }

    fn read_signed_leb_continuation(code: &[u8], cursor: &mut usize, first: u8) -> Result<i32> {
        let start = cursor.saturating_sub(1);
        let mut byte = first;
        while byte & 0x80 != 0 {
            byte = Self::read_byte(code, cursor)?;
        }
        let mut reader = BinaryReader::from_slice(&code[start..*cursor]);
        reader.read_sleb128().map_err(WasmError::from)
    }

    fn skip_bytes(code: &[u8], cursor: &mut usize, len: usize) -> Result<()> {
        if code.len().saturating_sub(*cursor) < len {
            return Err(WasmError::Validation(
                "unexpected end of immediate".to_string(),
            ));
        }
        *cursor += len;
        Ok(())
    }

    fn validate_start(&self, module: &Module) -> Result<()> {
        if let Some(start_idx) = module.start {
            let import_count = module
                .imports
                .iter()
                .filter(|i| matches!(i.kind, crate::runtime::ImportKind::Func(_)))
                .count() as u32;
            if start_idx >= import_count + module.funcs.len() as u32 {
                return Err(WasmError::Validation(
                    "start function index out of bounds".to_string(),
                ));
            }
            let func_type = module.func_type(start_idx).ok_or_else(|| {
                WasmError::Validation("start function has invalid type".to_string())
            })?;
            if !func_type.params.is_empty() || !func_type.results.is_empty() {
                return Err(WasmError::Validation(
                    "start function must have no params or results".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        DataKind, DataSegment, ElemKind, ElemSegment, ExportType, Func, FunctionType, GlobalType,
        Import, Limits, MemoryType, NumType, RefType, TableType, ValType,
    };

    #[test]
    fn test_validate_empty_module() {
        let module = Module::new();
        let validator = Validator::new();
        assert!(validator.validate(&module).is_ok());
    }

    #[test]
    fn test_validate_exported_imported_memory() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "memory".to_string(),
            kind: ImportKind::Memory(MemoryType::new(Limits::Min(1))),
        });
        module
            .exports
            .push(ExportType::new_memory("memory".to_string(), 0));

        let validator = Validator::new();
        assert!(validator.validate(&module).is_ok());
    }

    #[test]
    fn test_validate_rejects_invalid_import_function_type() {
        let mut module = Module::new();
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_non_import_const_global_get() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module
            .globals
            .push(GlobalType::new(ValType::Num(NumType::I32), false));
        module.global_inits.push(vec![0x41, 0x01, 0x0B]);
        module
            .globals
            .push(GlobalType::new(ValType::Num(NumType::I32), false));
        module.global_inits.push(vec![0x23, 0x00, 0x0B]);
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_duplicate_export_names() {
        let mut module = Module::new();
        module
            .exports
            .push(ExportType::new_memory("dup".to_string(), 0));
        module
            .exports
            .push(ExportType::new_memory("dup".to_string(), 0));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_multi_memory_modules() {
        let mut module = Module::new();
        module.memories.push(MemoryType::new(Limits::Min(1)));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_invalid_ref_null_type() {
        let mut module = Module::new();
        module.globals.push(GlobalType::new(
            ValType::Ref(crate::runtime::RefType::FuncRef),
            false,
        ));
        module.global_inits.push(vec![0xD0, 0x00, 0x0B]);

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_invalid_ref_null_in_function_body() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0xD0, 0x00, 0x1A, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_accepts_ref_is_null_in_unreachable_code() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x00, 0xD1, 0x1A, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_ok());
    }

    #[test]
    fn test_validate_rejects_invalid_local_index() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x20, 0x00, 0x1A, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_missing_function_result() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_bulk_memory_instruction() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0xFC, 0x0A, 0x00, 0x00, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_accepts_if_else_body() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0x04, 0x40, 0x05, 0x0B, 0x0B],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_ok());
    }

    #[test]
    fn test_validate_rejects_function_body_without_end() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_global_init_type_mismatch() {
        let mut module = Module::new();
        module
            .globals
            .push(GlobalType::new(ValType::Num(NumType::I64), false));
        module.global_inits.push(vec![0x41, 0x00, 0x0B]);

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_non_i32_data_offset() {
        let mut module = Module::new();
        module.memories.push(MemoryType::new(Limits::Min(1)));
        module.data.push(DataSegment {
            kind: DataKind::Active {
                memory_idx: 0,
                offset: vec![0x42, 0x00, 0x0B],
            },
            init: vec![1, 2, 3],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }

    #[test]
    fn test_validate_rejects_element_init_type_mismatch() {
        let mut module = Module::new();
        module
            .tables
            .push(TableType::new(RefType::FuncRef, Limits::Min(1)));
        module.elems.push(ElemSegment {
            kind: ElemKind::Active {
                table_idx: 0,
                offset: vec![0x41, 0x00, 0x0B],
            },
            type_: RefType::FuncRef,
            init: vec![vec![0xD0, 0x6F, 0x0B]],
        });

        let validator = Validator::new();
        assert!(validator.validate(&module).is_err());
    }
}
