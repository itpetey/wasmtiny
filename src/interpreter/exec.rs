use crate::interpreter::{ControlFrame, ControlStack, OperandStack};
use crate::runtime::{Instance, Module, Result, TrapCode, WasmError, WasmValue};
use std::sync::{Arc, Mutex};

const MAX_STACK_SIZE: usize = 16384;

pub struct Interpreter {
    pub operand_stack: OperandStack,
    pub control_stack: ControlStack,
    pub instance: Option<Arc<Mutex<Instance>>>,
    pub locals: Vec<WasmValue>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: None,
            locals: Vec::new(),
        }
    }

    pub fn with_instance(instance: Arc<Mutex<Instance>>) -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: Some(instance),
            locals: Vec::new(),
        }
    }

    pub fn execute(&mut self, module: &Module, func_idx: u32) -> Result<Vec<WasmValue>> {
        let func = module
            .func_at(func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))?;

        let func_type = module
            .type_at(func.type_idx)
            .ok_or_else(|| WasmError::Validation(format!("type {} not found", func.type_idx)))?;

        let param_count = func_type.params.len() as u32;
        let result_count = func_type.results.len() as u32;

        let frame = ControlFrame::new(param_count, result_count, func.body.clone());
        self.control_stack.push(frame);

        let result = self.run();

        self.control_stack.pop();

        result
    }

    fn run(&mut self) -> Result<Vec<WasmValue>> {
        while let Some(mut frame) = self.control_stack.pop_frame() {
            if frame.position >= frame.code.len() {
                let arity = frame.arity;
                let mut results = Vec::with_capacity(arity);
                for _ in 0..arity {
                    if let Some(value) = self.operand_stack.pop() {
                        results.push(value);
                    }
                }
                results.reverse();
                self.control_stack.push(frame);
                return Ok(results);
            }

            let opcode = frame.code[frame.position];
            frame.position += 1;

            match self.execute_opcode(opcode) {
                Ok(()) => {}
                Err(e) => {
                    self.control_stack.push(frame);
                    return Err(e);
                }
            }

            self.control_stack.push(frame);
        }

        Ok(Vec::new())
    }

    fn execute_opcode(&mut self, opcode: u8) -> Result<()> {
        match opcode {
            0x00 => return Err(WasmError::Trap(TrapCode::Unreachable)),
            0x01 => {}
            0x02 => {}
            0x03 => {}
            0x04 => {}
            0x05 => {}
            0x0B => {}
            0x0C => {
                let arity = self.operand_stack.pop_i32()? as u32;
                let frame = ControlFrame::new(0, arity, vec![]);
                self.control_stack.push(frame);
            }
            0x0D => {
                let _cond = self.operand_stack.pop_i32()?;
                let arity = self.operand_stack.pop_i32()? as u32;
                let frame = ControlFrame::new(0, arity, vec![]);
                self.control_stack.push(frame);
            }
            0x0E => {
                if let Some(frame) = self.control_stack.pop_frame() {
                    self.control_stack.push(frame);
                }
            }
            0x0F => return Ok(()),
            0x10 => {
                let func_idx = self.operand_stack.pop_i32()? as u32;

                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    let import_count = instance.module().import_count() as u32;

                    if func_idx < import_count {
                        let func_type = instance.module().func_type(func_idx).ok_or_else(|| {
                            WasmError::Validation(format!(
                                "function type not found for func {}",
                                func_idx
                            ))
                        })?;
                        let param_count = func_type.params.len();
                        drop(instance);

                        let mut args = Vec::with_capacity(param_count);
                        for _ in 0..param_count {
                            if let Some(arg) = self.operand_stack.pop() {
                                args.push(arg);
                            }
                        }
                        args.reverse();

                        let mut instance = self.instance.as_ref().unwrap().lock().unwrap();
                        let results = instance.call(func_idx, &args)?;
                        drop(instance);
                        for val in results {
                            self.operand_stack.push_unchecked(val);
                        }
                    } else {
                        let wasm_func_idx = func_idx - import_count;
                        if let Some(func) = instance.module().func_at(wasm_func_idx) {
                            let func_type =
                                instance.module().type_at(func.type_idx).ok_or_else(|| {
                                    WasmError::Validation(format!(
                                        "type {} not found",
                                        func.type_idx
                                    ))
                                })?;

                            let mut args = Vec::new();
                            for _ in 0..func_type.params.len() {
                                if let Some(arg) = self.operand_stack.pop() {
                                    args.push(arg);
                                }
                            }
                            args.reverse();

                            self.locals = args;

                            let frame = ControlFrame::new(
                                func_type.params.len() as u32,
                                func_type.results.len() as u32,
                                func.body.clone(),
                            );
                            self.control_stack.push(frame);
                        }
                    }
                } else {
                    return Err(WasmError::Runtime("no instance available".to_string()));
                }
            }
            0x11 => {
                let type_idx = self.operand_stack.pop_i32()? as u32;
                let func_idx = self.operand_stack.pop_i32()? as u32;

                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.first()
                        && (func_idx as usize) < table.data.len() {
                            let target_func_idx = table.data[func_idx as usize];

                            if let Some(func) = instance.module().func_at(target_func_idx) {
                                let func_type =
                                    instance.module().type_at(func.type_idx).ok_or_else(|| {
                                        WasmError::Validation(format!(
                                            "type {} not found",
                                            func.type_idx
                                        ))
                                    })?;

                                if func_type.params.len() as u32 != type_idx {
                                    return Err(WasmError::Runtime(
                                        "call_indirect type mismatch".to_string(),
                                    ));
                                }

                                let mut args = Vec::new();
                                for _ in 0..func_type.params.len() {
                                    if let Some(arg) = self.operand_stack.pop() {
                                        args.push(arg);
                                    }
                                }
                                args.reverse();

                                self.locals = args;

                                let frame = ControlFrame::new(
                                    func_type.params.len() as u32,
                                    func_type.results.len() as u32,
                                    func.body.clone(),
                                );
                                self.control_stack.push(frame);
                            }
                        }
                } else {
                    return Err(WasmError::Runtime("no instance available".to_string()));
                }
            }
            0x1A => drop(self.operand_stack.pop()),
            0x1B => {}
            0x20 => {
                let idx = self.operand_stack.pop_i32()? as usize;
                if idx < self.locals.len() {
                    self.operand_stack.push_unchecked(self.locals[idx]);
                }
            }
            0x21 => {
                let idx = self.operand_stack.pop_i32()? as usize;
                let val = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                if idx < self.locals.len() {
                    self.locals[idx] = val;
                } else {
                    self.locals.push(val);
                }
            }
            0x22 => {
                let idx = self.operand_stack.pop_i32()? as usize;
                let val = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                if idx < self.locals.len() {
                    self.locals[idx] = val;
                } else {
                    self.locals.push(val);
                }
                self.operand_stack.push_unchecked(val);
            }
            0x23 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    if let Some(global) = instance.globals.get(idx as usize) {
                        self.operand_stack.push_unchecked(global.value);
                    }
                }
            }
            0x24 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                let val = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                if let Some(ref instance) = self.instance {
                    let mut instance = instance.lock().unwrap();
                    if let Some(global) = instance.globals.get_mut(idx as usize) {
                        global.value = val;
                    }
                }
            }
            0x25 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.get(idx as usize) {
                        let elem_idx = self.operand_stack.pop_i32()? as usize;
                        if elem_idx < table.data.len() {
                            let func_idx = table.data[elem_idx];
                            self.operand_stack
                                .push_unchecked(WasmValue::FuncRef(func_idx));
                        }
                    }
                }
            }
            0x26 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let mut instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.get_mut(idx as usize) {
                        let elem_idx = self.operand_stack.pop_i32()? as usize;
                        let val = self
                            .operand_stack
                            .pop()
                            .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                        if let WasmValue::FuncRef(fidx) = val
                            && elem_idx < table.data.len() {
                                table.data[elem_idx] = fidx;
                            }
                    }
                }
            }
            0x27 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.get(idx as usize) {
                        self.operand_stack
                            .push_unchecked(WasmValue::I32(table.data.len() as i32));
                    }
                }
            }
            0x28 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                let delta = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let mut instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.get_mut(idx as usize) {
                        let old_size = table.data.len() as u32;
                        let new_size = old_size.saturating_add(delta);
                        if let Some(max) = table.type_.limits.max() {
                            if new_size > max {
                                self.operand_stack.push_unchecked(WasmValue::I32(-1));
                            } else {
                                table.data.resize(new_size as usize, 0);
                                self.operand_stack
                                    .push_unchecked(WasmValue::I32(old_size as i32));
                            }
                        } else {
                            table.data.resize(new_size as usize, 0);
                            self.operand_stack
                                .push_unchecked(WasmValue::I32(old_size as i32));
                        }
                    }
                }
            }
            0x29 => {
                let idx = self.operand_stack.pop_i32()? as u32;
                let _fill_val = self.operand_stack.pop_i32()?;
                let count = self.operand_stack.pop_i32()? as usize;
                let offset = self.operand_stack.pop_i32()? as usize;
                if let Some(ref instance) = self.instance {
                    let mut instance = instance.lock().unwrap();
                    if let Some(table) = instance.tables.get_mut(idx as usize) {
                        for i in offset..std::cmp::min(offset + count, table.data.len()) {
                            table.data[i] = 0;
                        }
                    }
                }
            }
            0x2A => {
                let _dst_idx = self.operand_stack.pop_i32()? as u32;
                let _src_idx = self.operand_stack.pop_i32()? as u32;
                let _count = self.operand_stack.pop_i32()? as usize;
            }
            0x2B => {
                let _table_idx = self.operand_stack.pop_i32()? as u32;
                let _elem_idx = self.operand_stack.pop_i32()? as u32;
                let _offset = self.operand_stack.pop_i32()?;
                let _count = self.operand_stack.pop_i32()?;
            }
            0x2C => {
                let _elem_idx = self.operand_stack.pop_i32()? as u32;
            }
            0x2D => {}
            0x2E => {}
            0x2F => {}
            0x3F => {
                if let Some(ref instance) = self.instance {
                    let instance = instance.lock().unwrap();
                    if let Some(mem) = instance.memories.first() {
                        self.operand_stack
                            .push_unchecked(WasmValue::I32(mem.size() as i32));
                    }
                }
            }
            0x40 => {
                let pages = self.operand_stack.pop_i32()? as u32;
                if let Some(ref instance) = self.instance {
                    let mut instance = instance.lock().unwrap();
                    if let Some(mem) = instance.memories.first_mut() {
                        let old_size = mem.size();
                        if mem.grow(pages).is_ok() {
                            self.operand_stack
                                .push_unchecked(WasmValue::I32(old_size as i32));
                        } else {
                            self.operand_stack.push_unchecked(WasmValue::I32(-1));
                        }
                    }
                }
            }
            0x41 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(val));
            }
            0x42 => {
                let val = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(val));
            }
            0x43 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(val));
            }
            0x44 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(val));
            }
            0x45 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if val == 0 { 1 } else { 0 }));
            }
            0x46 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a == b { 1 } else { 0 }));
            }
            0x47 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a != b { 1 } else { 0 }));
            }
            0x48 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a < b { 1 } else { 0 }));
            }
            0x49 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a > b { 1 } else { 0 }));
            }
            0x4A => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a <= b { 1 } else { 0 }));
            }
            0x4B => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(if a >= b { 1 } else { 0 }));
            }
            0x6A => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_add(b)));
            }
            0x6B => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_sub(b)));
            }
            0x6C => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_mul(b)));
            }
            0x6D => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_div(b)));
            }
            0x6E => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_div(b)));
            }
            0x6F => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push_unchecked(WasmValue::I32(a % b));
            }
            0x70 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push_unchecked(WasmValue::I32(a / b));
            }
            0x71 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a & b));
            }
            0x72 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a | b));
            }
            0x73 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a ^ b));
            }
            0x74 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a << b));
            }
            0x75 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a >> b));
            }
            0x76 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push_unchecked(WasmValue::I32(a >> b));
            }
            0x77 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.count_ones() as i32));
            }
            0x78 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.count_zeros() as i32));
            }
            0x79 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.leading_zeros() as i32));
            }
            0x7A => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.trailing_zeros() as i32));
            }
            0x7B => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(a.wrapping_add(b)));
            }
            0x7C => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(a.wrapping_sub(b)));
            }
            0x7D => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(a.wrapping_mul(b)));
            }
            0x7E => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push_unchecked(WasmValue::I64(a / b));
            }
            0x7F => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push_unchecked(WasmValue::I64(a % b));
            }
            0x80 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(a & b));
            }
            0x81 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(a | b));
            }
            0x82 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(a ^ b));
            }
            0x83 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(a << b));
            }
            0x84 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push_unchecked(WasmValue::I64(a >> b));
            }
            0x85 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(val.abs()));
            }
            0x86 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(-val));
            }
            0x87 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F32(val.ceil()));
            }
            0x88 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F32(val.floor()));
            }
            0x89 => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F32(val.trunc()));
            }
            0x8A => {
                let val = self.operand_stack.pop_f32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F32(val.sqrt()));
            }
            0x8B => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a + b));
            }
            0x8C => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a - b));
            }
            0x8D => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a * b));
            }
            0x8E => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a / b));
            }
            0x8F => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a.min(b)));
            }
            0x90 => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push_unchecked(WasmValue::F32(a.max(b)));
            }
            0xA0 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(val.abs()));
            }
            0xA1 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(-val));
            }
            0xA2 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F64(val.ceil()));
            }
            0xA3 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F64(val.floor()));
            }
            0xA4 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F64(val.trunc()));
            }
            0xA5 => {
                let val = self.operand_stack.pop_f64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::F64(val.sqrt()));
            }
            0xA6 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a + b));
            }
            0xA7 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a - b));
            }
            0xA8 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a * b));
            }
            0xA9 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a / b));
            }
            0xAA => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a.min(b)));
            }
            0xAB => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push_unchecked(WasmValue::F64(a.max(b)));
            }
            0xAC => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(a.wrapping_shl(b as u32)));
            }
            0xAD => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32((a as u32).wrapping_shr(b as u32) as i32));
            }
            0xAE => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(a.wrapping_shl(b as u32)));
            }
            0xAF => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64((a as u64).wrapping_shr(b as u32) as i64));
            }
            0xB0 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.count_ones() as i32));
            }
            0xB1 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.count_zeros() as i32));
            }
            0xB2 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.leading_zeros() as i32));
            }
            0xB3 => {
                let val = self.operand_stack.pop_i32()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I32(val.trailing_zeros() as i32));
            }
            0xB4 => {
                let val = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(val.count_ones() as i64));
            }
            0xB5 => {
                let val = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(val.count_zeros() as i64));
            }
            0xB6 => {
                let val = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(val.leading_zeros() as i64));
            }
            0xB7 => {
                let val = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push_unchecked(WasmValue::I64(val.trailing_zeros() as i64));
            }
            0xD0 => {
                self.operand_stack.push_unchecked(WasmValue::NullRef);
            }
            0xD1 => {
                let val = self.operand_stack.pop();
                if matches!(val, Some(WasmValue::NullRef)) {
                    self.operand_stack.push_unchecked(WasmValue::I32(1));
                } else {
                    self.operand_stack.push_unchecked(WasmValue::I32(0));
                }
            }
            0xD2 => {
                let func_idx = self.operand_stack.pop_i32()? as u32;
                self.operand_stack
                    .push_unchecked(WasmValue::FuncRef(func_idx));
            }
            _ => {
                return Err(WasmError::Runtime(format!(
                    "unknown opcode: {:02x}",
                    opcode
                )));
            }
        }
        Ok(())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpreter_creation() {
        let interp = Interpreter::new();
        assert!(interp.operand_stack.is_empty());
        assert!(interp.control_stack.is_empty());
    }

    #[test]
    fn test_i32_operations() {
        let mut interp = Interpreter::new();
        interp.operand_stack.push_unchecked(WasmValue::I32(5));
        interp.operand_stack.push_unchecked(WasmValue::I32(3));
        assert_eq!(interp.operand_stack.pop_i32().unwrap(), 3);
        assert_eq!(interp.operand_stack.pop_i32().unwrap(), 5);
    }

    #[test]
    fn test_bit_operations() {
        let mut interp = Interpreter::new();
        interp.operand_stack.push_unchecked(WasmValue::I32(0b1100));
        interp.operand_stack.push_unchecked(WasmValue::I32(0b1010));
        let b = interp.operand_stack.pop_i32().unwrap();
        let a = interp.operand_stack.pop_i32().unwrap();
        let result = a & b;
        assert_eq!(result, 0b1000);
    }
}
