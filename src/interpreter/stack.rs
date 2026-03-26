use crate::runtime::WasmValue;

#[derive(Debug, Clone)]
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

    pub fn from_vec(slots: Vec<WasmValue>, max_size: usize) -> Self {
        Self { slots, max_size }
    }

    pub fn max_size(&self) -> usize {
        self.max_size
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

    pub fn truncate(&mut self, len: usize) {
        self.slots.truncate(len);
    }

    pub fn to_vec(&self) -> Vec<WasmValue> {
        self.slots.clone()
    }
}

#[derive(Debug, Clone)]
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

    pub fn last(&self) -> Option<&ControlFrame> {
        self.frames.last()
    }

    pub fn last_mut(&mut self) -> Option<&mut ControlFrame> {
        self.frames.last_mut()
    }

    pub fn frames(&self) -> &[ControlFrame] {
        &self.frames
    }

    pub fn truncate(&mut self, len: usize) {
        self.frames.truncate(len);
    }

    pub fn get(&self, idx: usize) -> Option<&ControlFrame> {
        self.frames.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut ControlFrame> {
        self.frames.get_mut(idx)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        use std::io::Write;
        let mut bytes = Vec::new();
        for frame in &self.frames {
            bytes
                .write_all(&(frame.position as u32).to_le_bytes())
                .unwrap();
            bytes
                .write_all(&(frame.arity as u32).to_le_bytes())
                .unwrap();
            bytes
                .write_all(&(frame.label_arity as u32).to_le_bytes())
                .unwrap();
            bytes
                .write_all(&(frame.height as u32).to_le_bytes())
                .unwrap();
            bytes
                .write_all(&(frame.local_count as u32).to_le_bytes())
                .unwrap();
            bytes.push(frame.kind as u8);
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < bytes.len() {
            if cursor + 21 > bytes.len() {
                break;
            }
            let position = u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]) as usize;
            let arity = u32::from_le_bytes([
                bytes[cursor + 4],
                bytes[cursor + 5],
                bytes[cursor + 6],
                bytes[cursor + 7],
            ]) as usize;
            let label_arity = u32::from_le_bytes([
                bytes[cursor + 8],
                bytes[cursor + 9],
                bytes[cursor + 10],
                bytes[cursor + 11],
            ]) as usize;
            let height = u32::from_le_bytes([
                bytes[cursor + 12],
                bytes[cursor + 13],
                bytes[cursor + 14],
                bytes[cursor + 15],
            ]) as usize;
            let local_count = u32::from_le_bytes([
                bytes[cursor + 16],
                bytes[cursor + 17],
                bytes[cursor + 18],
                bytes[cursor + 19],
            ]) as usize;
            let kind = FrameKind::from_u8(bytes[cursor + 20]);
            cursor += 21;

            frames.push(ControlFrame {
                kind,
                position,
                code: Vec::new(),
                arity,
                label_arity,
                local_count,
                height,
                locals: Vec::new(),
            });
        }
        Self { frames }
    }
}

impl Default for ControlStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FrameKind {
    Function,
    Block,
    Loop,
}

impl FrameKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => FrameKind::Function,
            1 => FrameKind::Block,
            2 => FrameKind::Loop,
            _ => FrameKind::Block,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlFrame {
    pub kind: FrameKind,
    pub position: usize,
    pub code: Vec<u8>,
    pub arity: usize,
    pub label_arity: usize,
    pub local_count: usize,
    pub height: usize,
    pub locals: Vec<WasmValue>,
}

impl ControlFrame {
    pub fn new(
        kind: FrameKind,
        param_count: u32,
        result_count: u32,
        label_count: u32,
        code: Vec<u8>,
        locals: Vec<WasmValue>,
    ) -> Self {
        Self {
            kind,
            position: 0,
            code,
            arity: result_count as usize,
            label_arity: label_count as usize,
            local_count: param_count as usize,
            height: 0,
            locals,
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
        let frame = ControlFrame::new(FrameKind::Function, 0, 0, 0, vec![], vec![]);
        stack.push(frame);
        assert_eq!(stack.len(), 1);
    }
}
