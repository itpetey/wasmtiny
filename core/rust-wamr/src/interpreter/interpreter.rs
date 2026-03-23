use crate::interpreter::{ControlFrame, ControlStack, OperandStack};
use crate::runtime::{Instance, Module, Result, TrapCode, WasmError, WasmValue};
use std::sync::{Arc, Mutex};

const MAX_STACK_SIZE: usize = 16384;

pub struct Interpreter {
    pub operand_stack: OperandStack,
    pub control_stack: ControlStack,
    pub instance: Option<Arc<Mutex<Instance>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: None,
        }
    }

    pub fn with_instance(instance: Arc<Mutex<Instance>>) -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: Some(instance),
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

            match self.execute_opcode(opcode, &mut frame) {
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

    fn execute_opcode(&mut self, opcode: u8, frame: &mut ControlFrame) -> Result<()> {
        match opcode {
            0x41 => {
                let c = frame.get_i32(&mut self.operand_stack)?;
                self.operand_stack.push(WasmValue::I32(c));
            }
            0x42 => {
                let c = frame.get_i64(&mut self.operand_stack)?;
                self.operand_stack.push(WasmValue::I64(c));
            }
            0x43 => {
                let c = frame.get_f32(&mut self.operand_stack)?;
                self.operand_stack.push(WasmValue::F32(c));
            }
            0x44 => {
                let c = frame.get_f64(&mut self.operand_stack)?;
                self.operand_stack.push(WasmValue::F64(c));
            }
            0x6A => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_add(b)));
            }
            0x6B => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_sub(b)));
            }
            0x6C => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_mul(b)));
            }
            0x70 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push(WasmValue::I32(a / b));
            }
            0x1A => {
                self.operand_stack.pop();
            }
            0x0B => {}
            0x0F => {
                return Ok(());
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
}
