use crate::runtime::{Module, Result, WasmError, WasmValue};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum IrOpcode {
    LoadConst {
        dst: u8,
        value: i64,
    },
    LoadGlobal {
        dst: u8,
        idx: u32,
    },
    StoreGlobal {
        src: u8,
        idx: u32,
    },
    LoadLocal {
        dst: u8,
        idx: u8,
    },
    StoreLocal {
        src: u8,
        idx: u8,
    },
    Add {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Sub {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Mul {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Div {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    And {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Or {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Xor {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Shl {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Shr {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Lt {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Le {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Eq {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Ne {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Ge {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Gt {
        dst: u8,
        lhs: u8,
        rhs: u8,
    },
    Branch {
        target: u32,
    },
    BranchIf {
        cond: u8,
        target: u32,
    },
    Call {
        func_idx: u32,
        args: Vec<u8>,
        result: Option<u8>,
    },
    Return {
        values: Vec<u8>,
    },
    LoadMem {
        dst: u8,
        base: u8,
        offset: i32,
    },
    StoreMem {
        src: u8,
        base: u8,
        offset: i32,
    },
    Nop,
}

pub struct IrBlock {
    pub instructions: Vec<IrOpcode>,
    pub successors: Vec<u32>,
}

impl IrBlock {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            successors: Vec::new(),
        }
    }
}

impl Default for IrBlock {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FastInterpreter {
    registers: Vec<WasmValue>,
    blocks: HashMap<u32, IrBlock>,
    #[allow(dead_code)]
    current_block: u32,
    local_count: u8,
}

impl FastInterpreter {
    pub fn new() -> Self {
        Self {
            registers: Vec::with_capacity(256),
            blocks: HashMap::new(),
            current_block: 0,
            local_count: 0,
        }
    }

    pub fn compile_module(&mut self, module: &Module) -> Result<()> {
        for (func_idx, _func) in module.funcs.iter().enumerate() {
            self.compile_function(module, func_idx as u32)?;
        }
        Ok(())
    }

    fn compile_function(&mut self, module: &Module, func_idx: u32) -> Result<()> {
        let func = module
            .func_at(func_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))?;

        let mut block = IrBlock::new();
        let mut register_map: HashMap<u32, u8> = HashMap::new();
        let mut next_reg = 0u8;

        let mut i = 0;
        while i < func.body.len() {
            let opcode = func.body[i];
            match opcode {
                0x20 => {
                    let idx = func.body[i + 1];
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    register_map.insert(idx as u32, dst);
                    block.instructions.push(IrOpcode::LoadLocal { dst, idx });
                    i += 2;
                }
                0x21 => {
                    let idx = func.body[i + 1];
                    let src = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::StoreLocal { src, idx });
                    i += 2;
                }
                0x23 => {
                    let idx = u32::from_le_bytes([
                        func.body[i + 1],
                        func.body[i + 2],
                        func.body[i + 3],
                        func.body[i + 4],
                    ]);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::LoadGlobal { dst, idx });
                    i += 5;
                }
                0x24 => {
                    let idx = u32::from_le_bytes([
                        func.body[i + 1],
                        func.body[i + 2],
                        func.body[i + 3],
                        func.body[i + 4],
                    ]);
                    let src = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::StoreGlobal { src, idx });
                    i += 5;
                }
                0x6A => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Add {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x6B => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Sub {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x6C => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Mul {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x6D => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Div {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x71 => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Eq {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x72 => {
                    let b = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let a = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    let dst = next_reg;
                    next_reg = next_reg.wrapping_add(1);
                    block.instructions.push(IrOpcode::Ne {
                        dst,
                        lhs: a,
                        rhs: b,
                    });
                    i += 1;
                }
                0x10 => {
                    let func_idx = func.body[i + 1] as u32;
                    block.instructions.push(IrOpcode::Call {
                        func_idx,
                        args: vec![],
                        result: None,
                    });
                    i += 2;
                }
                0x0F => {
                    block.instructions.push(IrOpcode::Return { values: vec![] });
                    break;
                }
                0x00 => {
                    block.instructions.push(IrOpcode::Branch { target: 0 });
                    break;
                }
                _ => {
                    block.instructions.push(IrOpcode::Nop);
                    i += 1;
                }
            }
        }

        self.blocks.insert(func_idx, block);
        self.local_count = self.local_count.max(next_reg);
        Ok(())
    }

    pub fn execute(&mut self, func_idx: u32) -> Result<Vec<WasmValue>> {
        let block = self.blocks.get(&func_idx).ok_or_else(|| {
            WasmError::Runtime(format!("compiled function {} not found", func_idx))
        })?;

        self.registers
            .resize(self.local_count as usize + 16, WasmValue::I32(0));

        for ir in &block.instructions {
            match ir {
                IrOpcode::Nop => {}
                IrOpcode::LoadConst { dst, value } => {
                    self.registers[*dst as usize] = WasmValue::I32(*value as i32);
                }
                IrOpcode::LoadGlobal { dst, idx: _ } => {
                    self.registers[*dst as usize] = WasmValue::I32(0);
                }
                IrOpcode::StoreGlobal { src: _, idx: _ } => {}
                IrOpcode::LoadLocal { dst, idx } => {
                    if (*idx as usize) < self.registers.len() {
                        self.registers[*dst as usize] = self.registers[*idx as usize];
                    }
                }
                IrOpcode::StoreLocal { src, idx } => {
                    if (*idx as usize) < self.registers.len() {
                        self.registers[*idx as usize] = self.registers[*src as usize];
                    }
                }
                IrOpcode::Add { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32((a + b) as i32);
                }
                IrOpcode::Sub { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32((a - b) as i32);
                }
                IrOpcode::Mul { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32((a * b) as i32);
                }
                IrOpcode::Div { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v as i64,
                        _ => 0,
                    };
                    if b != 0 {
                        self.registers[*dst as usize] = WasmValue::I32((a / b) as i32);
                    }
                }
                IrOpcode::And { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32(a & b);
                }
                IrOpcode::Or { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32(a | b);
                }
                IrOpcode::Xor { dst, lhs, rhs } => {
                    let a = match &self.registers[*lhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    let b = match &self.registers[*rhs as usize] {
                        WasmValue::I32(v) => *v,
                        _ => 0,
                    };
                    self.registers[*dst as usize] = WasmValue::I32(a ^ b);
                }
                IrOpcode::Eq { dst, lhs, rhs } => {
                    let a = &self.registers[*lhs as usize];
                    let b = &self.registers[*rhs as usize];
                    let result = a == b;
                    self.registers[*dst as usize] = WasmValue::I32(if result { 1 } else { 0 });
                }
                IrOpcode::Ne { dst, lhs, rhs } => {
                    let a = &self.registers[*lhs as usize];
                    let b = &self.registers[*rhs as usize];
                    let result = a != b;
                    self.registers[*dst as usize] = WasmValue::I32(if result { 1 } else { 0 });
                }
                IrOpcode::Branch { target: _ } => break,
                IrOpcode::BranchIf { cond, target: _ } => {
                    let c = match &self.registers[*cond as usize] {
                        WasmValue::I32(v) => *v != 0,
                        _ => false,
                    };
                    if c {
                        break;
                    }
                }
                IrOpcode::Call {
                    func_idx: _,
                    args: _,
                    result: Some(dst),
                } => {
                    self.registers[*dst as usize] = WasmValue::I32(42);
                }
                IrOpcode::Return { values } => {
                    let mut results = Vec::new();
                    for v in values {
                        results.push(self.registers[*v as usize]);
                    }
                    return Ok(results);
                }
                _ => {}
            }
        }

        Ok(vec![])
    }
}

impl Default for FastInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_interpreter_creation() {
        let interp = FastInterpreter::new();
        assert!(interp.registers.is_empty());
        assert!(interp.blocks.is_empty());
    }

    #[test]
    fn test_ir_opcode_variants() {
        let op = IrOpcode::Add {
            dst: 0,
            lhs: 1,
            rhs: 2,
        };
        assert!(matches!(op, IrOpcode::Add { .. }));
    }

    #[test]
    fn test_ir_block() {
        let mut block = IrBlock::new();
        block.instructions.push(IrOpcode::Nop);
        block.successors.push(1);
        assert_eq!(block.instructions.len(), 1);
        assert_eq!(block.successors.len(), 1);
    }
}
