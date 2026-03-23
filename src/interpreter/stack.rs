use crate::runtime::WasmValue;

pub struct OperandStack {
    slots: Vec<WasmValue>,
    max_size: usize,
}

impl OperandStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            slots: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, value: WasmValue) -> crate::runtime::Result<()> {
        if self.slots.len() >= self.max_size {
            return Err(crate::runtime::WasmError::Runtime("stack overflow".into()));
        }
        self.slots.push(value);
        Ok(())
    }

    pub fn push_unchecked(&mut self, value: WasmValue) {
        debug_assert!(
            self.slots.len() < self.max_size,
            "stack overflow (validated module should not reach this)"
        );
        self.slots.push(value);
    }

    pub fn pop(&mut self) -> Option<WasmValue> {
        self.slots.pop()
    }

    pub fn pop_i32(&mut self) -> crate::runtime::Result<i32> {
        match self.pop() {
            Some(WasmValue::I32(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn pop_i64(&mut self) -> crate::runtime::Result<i64> {
        match self.pop() {
            Some(WasmValue::I64(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn pop_f32(&mut self) -> crate::runtime::Result<f32> {
        match self.pop() {
            Some(WasmValue::F32(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn pop_f64(&mut self) -> crate::runtime::Result<f64> {
        match self.pop() {
            Some(WasmValue::F64(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

pub struct ControlStack {
    frames: Vec<ControlFrame>,
}

impl ControlStack {
    pub fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub fn push(&mut self, frame: ControlFrame) {
        self.frames.push(frame);
    }

    pub fn pop(&mut self) -> Option<ControlFrame> {
        self.frames.pop()
    }

    pub fn pop_frame(&mut self) -> Option<ControlFrame> {
        self.frames.pop()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

impl Default for ControlStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ControlFrame {
    pub position: usize,
    pub code: Vec<u8>,
    pub arity: usize,
    pub local_count: usize,
    pub height: usize,
}

impl ControlFrame {
    pub fn new(param_count: u32, result_count: u32, code: Vec<u8>) -> Self {
        Self {
            position: 0,
            code,
            arity: result_count as usize,
            local_count: param_count as usize,
            height: 0,
        }
    }

    pub fn get_i32(&self, stack: &mut OperandStack) -> crate::runtime::Result<i32> {
        let idx = match stack.pop() {
            Some(WasmValue::I32(v)) => v as u32,
            Some(_) => {
                return Err(crate::runtime::WasmError::Runtime(
                    "type mismatch".to_string(),
                ));
            }
            None => {
                return Err(crate::runtime::WasmError::Runtime(
                    "stack underflow".to_string(),
                ));
            }
        };
        Ok(idx as i32)
    }

    pub fn get_i64(&self, stack: &mut OperandStack) -> crate::runtime::Result<i64> {
        match stack.pop() {
            Some(WasmValue::I64(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn get_f32(&self, stack: &mut OperandStack) -> crate::runtime::Result<f32> {
        match stack.pop() {
            Some(WasmValue::F32(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }

    pub fn get_f64(&self, stack: &mut OperandStack) -> crate::runtime::Result<f64> {
        match stack.pop() {
            Some(WasmValue::F64(v)) => Ok(v),
            Some(_) => Err(crate::runtime::WasmError::Runtime(
                "type mismatch".to_string(),
            )),
            None => Err(crate::runtime::WasmError::Runtime(
                "stack underflow".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_stack() {
        let mut stack = OperandStack::new(100);
        stack.push_unchecked(WasmValue::I32(42));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.pop(), Some(WasmValue::I32(42)));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_control_stack() {
        let mut stack = ControlStack::new();
        let frame = ControlFrame::new(0, 0, vec![]);
        stack.push(frame);
        assert_eq!(stack.len(), 1);
    }
}
