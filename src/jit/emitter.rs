#[allow(unused_imports)]
#[allow(dead_code)]
use crate::runtime::{Result, WasmError};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemOffset {
    Disp8(i8),
    Disp32(i32),
    RipRel(i32),
}

#[allow(dead_code)]
impl MemOffset {
    pub fn encode(&self, code: &mut Vec<u8>, base: Option<Reg>, index: Option<Reg>, _scale: u8) {
        match self {
            MemOffset::Disp8(disp) => {
                code.push(
                    0x40 | (index.map(|i| i.encode() << 3).unwrap_or(0))
                        | base.map(|r| r.encode()).unwrap_or(0),
                );
                code.push(*disp as u8);
            }
            MemOffset::Disp32(disp) => {
                code.push(
                    0x80 | (index.map(|i| i.encode() << 3).unwrap_or(0))
                        | base.map(|r| r.encode()).unwrap_or(0),
                );
                code.extend_from_slice(&disp.to_le_bytes());
            }
            MemOffset::RipRel(disp) => {
                code.push(0x05);
                code.extend_from_slice(&disp.to_le_bytes());
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Address {
    pub base: Option<Reg>,
    pub index: Option<Reg>,
    pub scale: u8,
    pub displacement: Option<i32>,
}

impl Address {
    pub fn new(base: Reg) -> Self {
        Self {
            base: Some(base),
            index: None,
            scale: 1,
            displacement: None,
        }
    }

    pub fn with_displacement(mut self, disp: i32) -> Self {
        self.displacement = Some(disp);
        self
    }

    pub fn with_index(mut self, index: Reg, scale: u8) -> Self {
        self.index = Some(index);
        self.scale = scale;
        self
    }

    pub fn emit_modrm(&self, code: &mut Vec<u8>, _reg_enc: u8) {
        let base_enc = self.base.map(|r| r.encode()).unwrap_or(0x05);
        let index_enc = self.index.map(|r| r.encode()).unwrap_or(0x04);

        let scaled_index = match self.scale {
            1 => 0x00,
            2 => 0x40,
            4 => 0x80,
            8 => 0xC0,
            _ => 0x00,
        };

        if self.displacement.is_none() && self.base != Some(Reg::Rsp) {
            code.push(scaled_index | base_enc);
            if self.index.is_some() {
                code.push(0x24 | (index_enc << 3) | base_enc);
            }
        } else if let Some(disp) = self.displacement {
            if disp >= i8::MIN as i32 && disp <= i8::MAX as i32 {
                code.push(0x40 | scaled_index | base_enc);
            } else {
                code.push(0x80 | scaled_index | base_enc);
            }
            if self.index.is_some() {
                code.push(0x24 | (index_enc << 3) | base_enc);
            }
            code.extend_from_slice(&disp.to_le_bytes()[..4]);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reg {
    Rax,
    Rcx,
    Rdx,
    Rbx,
    Rsp,
    Rbp,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Al,
    Cl,
    Dl,
    Bl,
}

impl Reg {
    pub fn encode(self) -> u8 {
        match self {
            Reg::Rax | Reg::Al => 0,
            Reg::Rcx | Reg::Cl => 1,
            Reg::Rdx | Reg::Dl => 2,
            Reg::Rbx | Reg::Bl => 3,
            Reg::Rsp => 4,
            Reg::Rbp => 5,
            Reg::Rsi => 6,
            Reg::Rdi => 7,
            Reg::R8 => 8,
            Reg::R9 => 9,
            Reg::R10 => 10,
            Reg::R11 => 11,
            Reg::R12 => 12,
            Reg::R13 => 13,
            Reg::R14 => 14,
            Reg::R15 => 15,
        }
    }

    pub fn is_64bit(self) -> bool {
        !self.is_8bit()
    }

    pub fn is_8bit(self) -> bool {
        matches!(self, Reg::Al | Reg::Cl | Reg::Dl | Reg::Bl)
    }

    pub fn word_size(self) -> u8 {
        if self.is_64bit() { 8 } else { 4 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XmmReg {
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
}

impl XmmReg {
    pub fn encode(self) -> u8 {
        match self {
            XmmReg::Xmm0 => 0,
            XmmReg::Xmm1 => 1,
            XmmReg::Xmm2 => 2,
            XmmReg::Xmm3 => 3,
            XmmReg::Xmm4 => 4,
            XmmReg::Xmm5 => 5,
            XmmReg::Xmm6 => 6,
            XmmReg::Xmm7 => 7,
            XmmReg::Xmm8 => 8,
            XmmReg::Xmm9 => 9,
            XmmReg::Xmm10 => 10,
            XmmReg::Xmm11 => 11,
            XmmReg::Xmm12 => 12,
            XmmReg::Xmm13 => 13,
            XmmReg::Xmm14 => 14,
            XmmReg::Xmm15 => 15,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Overflow,
    NotOverflow,
    Below,
    AboveOrEqual,
    Equal,
    NotEqual,
    BelowOrEqual,
    Above,
    Sign,
    NotSign,
    Parity,
    NotParity,
    LessSigned,
    GreaterOrEqualSigned,
    LessOrEqualSigned,
    GreaterSigned,
}

impl Condition {
    pub fn encode(self) -> u8 {
        match self {
            Condition::Overflow => 0x0,
            Condition::NotOverflow => 0x1,
            Condition::Below => 0x2,
            Condition::AboveOrEqual => 0x3,
            Condition::Equal => 0x4,
            Condition::NotEqual => 0x5,
            Condition::BelowOrEqual => 0x6,
            Condition::Above => 0x7,
            Condition::Sign => 0x8,
            Condition::NotSign => 0x9,
            Condition::Parity => 0xA,
            Condition::NotParity => 0xB,
            Condition::LessSigned => 0xC,
            Condition::GreaterOrEqualSigned => 0xD,
            Condition::LessOrEqualSigned => 0xE,
            Condition::GreaterSigned => 0xF,
        }
    }
}

pub struct Emitter {
    code: Vec<u8>,
}

impl Emitter {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            code: Vec::with_capacity(capacity),
        }
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }

    pub fn code_mut(&mut self) -> &mut Vec<u8> {
        &mut self.code
    }

    pub fn take_code(self) -> Vec<u8> {
        self.code
    }

    pub fn emit_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn emit_rex(&mut self, w: bool, x: bool, b: bool, r: bool) {
        let byte = 0x40
            | (if w { 0x08 } else { 0 })
            | (if x { 0x04 } else { 0 })
            | (if b { 0x01 } else { 0 })
            | (if r { 0x02 } else { 0 });
        self.code.push(byte);
    }

    pub fn emit_modrm(&mut self, mod_: u8, rm: u8, reg: u8) {
        self.code.push((mod_ << 6) | (reg << 3) | rm);
    }

    pub fn emit_mov_rr(&mut self, dst: Reg, src: Reg) {
        if dst.is_8bit() || src.is_8bit() {
            self.code.push(0x88);
        } else {
            let need_rex = dst.encode() >= 8 || src.encode() >= 8;
            self.emit_rex(dst.is_64bit(), false, need_rex, need_rex);
            self.code.push(0x89);
        }
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_mov_ra(&mut self, dst: Reg, imm: u64) {
        let opcode = if dst.is_8bit() { 0xB0 } else { 0xB8 };
        let enc = dst.encode();

        if dst.is_64bit() {
            self.emit_rex(true, false, false, false);
        } else if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        }

        self.code.push(opcode | enc);

        if dst.is_64bit() {
            self.code.extend_from_slice(&imm.to_le_bytes());
        } else {
            self.code.extend_from_slice(&(imm as u32).to_le_bytes());
        }
    }

    pub fn emit_mov_rm(&mut self, dst: Reg, addr: &Address) {
        if dst.is_8bit() {
            self.code.push(0x8A);
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), false);
            self.code.push(0x8B);
        }
        addr.emit_modrm(&mut self.code, dst.encode());

        if let Some(disp) = addr.displacement {
            self.code.extend_from_slice(&disp.to_le_bytes());
        }
    }

    pub fn emit_mov_mr(&mut self, addr: &Address, src: Reg) {
        if src.is_8bit() {
            self.code.push(0x88);
        } else {
            self.emit_rex(src.is_64bit(), false, false, src.is_64bit());
            self.code.push(0x89);
        }
        addr.emit_modrm(&mut self.code, src.encode());

        if let Some(disp) = addr.displacement {
            self.code.extend_from_slice(&disp.to_le_bytes());
        }
    }

    pub fn emit_add_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x00
        } else {
            0x01
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            let need_rex = dst.encode() >= 8 || src.encode() >= 8;
            self.emit_rex(dst.is_64bit(), false, need_rex, need_rex);
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_sub_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x28
        } else {
            0x29
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            let need_rex = dst.encode() >= 8 || src.encode() >= 8;
            self.emit_rex(dst.is_64bit(), false, need_rex, need_rex);
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_mul_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x0A
        } else {
            0x0B
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, dst.is_64bit(), src.is_64bit());
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_xor_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x30
        } else {
            0x31
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, dst.is_64bit(), src.is_64bit());
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_or_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x08
        } else {
            0x09
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, dst.is_64bit(), src.is_64bit());
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_and_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x20
        } else {
            0x21
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, dst.is_64bit(), src.is_64bit());
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_cmp_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x38
        } else {
            0x39
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            let need_rex = dst.encode() >= 8 || src.encode() >= 8;
            self.emit_rex(dst.is_64bit(), false, need_rex, need_rex);
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_test_rr(&mut self, dst: Reg, src: Reg) {
        let opcode = if dst.is_8bit() || src.is_8bit() {
            0x84
        } else {
            0x85
        };
        if dst.is_8bit() || src.is_8bit() {
            self.emit_rex(false, false, dst.is_64bit(), src.is_64bit());
        } else {
            self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        }
        self.code.push(opcode);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_shl_ri(&mut self, dst: Reg, imm: u8) {
        let opcode = if dst.is_8bit() { 0xC0 } else { 0xC1 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x04, dst.encode());
        self.code.push(imm & 0x1F);
    }

    pub fn emit_shr_ri(&mut self, dst: Reg, imm: u8) {
        let opcode = if dst.is_8bit() { 0xC0 } else { 0xC1 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x05, dst.encode());
        self.code.push(imm & 0x1F);
    }

    pub fn emit_sar_ri(&mut self, dst: Reg, imm: u8) {
        let opcode = if dst.is_8bit() { 0xC0 } else { 0xC1 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x07, dst.encode());
        self.code.push(imm & 0x1F);
    }

    pub fn emit_rol_ri(&mut self, dst: Reg, imm: u8) {
        let opcode = if dst.is_8bit() { 0xC0 } else { 0xC1 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x00, dst.encode());
        self.code.push(imm & 0x1F);
    }

    pub fn emit_ror_ri(&mut self, dst: Reg, imm: u8) {
        let opcode = if dst.is_8bit() { 0xC0 } else { 0xC1 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x01, dst.encode());
        self.code.push(imm & 0x1F);
    }

    pub fn emit_not(&mut self, dst: Reg) {
        let opcode = if dst.is_8bit() { 0xF6 } else { 0xF7 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x02, dst.encode());
    }

    pub fn emit_neg(&mut self, dst: Reg) {
        let opcode = if dst.is_8bit() { 0xF6 } else { 0xF7 };
        self.emit_rex(dst.is_64bit(), false, false, false);
        self.code.push(opcode);
        self.emit_modrm(0x03, 0x03, dst.encode());
    }

    pub fn emit_jmp_rel32(&mut self, offset: i32) {
        self.code.push(0xE9);
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    pub fn emit_jmp_rel8(&mut self, offset: i8) {
        self.code.push(0xEB);
        self.code.push(offset as u8);
    }

    pub fn emit_jcc_rel32(&mut self, cond: Condition, offset: i32) {
        self.code.push(0x0F);
        self.code.push(0x80 | cond.encode());
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    pub fn emit_jcc_rel8(&mut self, cond: Condition, offset: i8) {
        self.code.push(0x70 | cond.encode());
        self.code.push(offset as u8);
    }

    pub fn emit_call_rel32(&mut self, offset: i32) {
        self.code.push(0xE8);
        self.code.extend_from_slice(&offset.to_le_bytes());
    }

    pub fn emit_ret(&mut self) {
        self.code.push(0xC3);
    }

    pub fn emit_ret_imm(&mut self, imm: u16) {
        self.code.push(0xC2);
        self.code.extend_from_slice(&imm.to_le_bytes());
    }

    pub fn emit_pop(&mut self, dst: Reg) {
        if dst.is_64bit() && dst != Reg::Rsp && dst != Reg::Rbp {
            self.emit_rex(true, false, false, false);
        }
        self.code.push(0x58 | dst.encode());
    }

    pub fn emit_push(&mut self, src: Reg) {
        if src.is_64bit() && src != Reg::Rsp && src != Reg::Rbp {
            self.emit_rex(true, false, false, false);
        }
        self.code.push(0x50 | src.encode());
    }

    pub fn emit_push_imm32(&mut self, imm: i32) {
        self.code.push(0x68);
        self.code.extend_from_slice(&imm.to_le_bytes());
    }

    pub fn emit_push_imm64(&mut self, imm: i64) {
        self.code.push(0x68);
        self.code.extend_from_slice(&imm.to_le_bytes()[..4]);
    }

    pub fn emit_cdq(&mut self) {
        self.code.push(0x99);
    }

    pub fn emit_cqo(&mut self) {
        self.emit_rex(true, false, false, false);
        self.code.push(0x99);
    }

    pub fn emit_movzx_rr(&mut self, dst: Reg, src: Reg) {
        let src_size = if src.is_8bit() { 0xB6 } else { 0xB7 };
        self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        self.code.push(0x0F);
        self.code.push(src_size);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_movsx_rr(&mut self, dst: Reg, src: Reg) {
        let src_size = if src.is_8bit() { 0xBE } else { 0xBF };
        self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), src.is_64bit());
        self.code.push(0x0F);
        self.code.push(src_size);
        self.emit_modrm(0x03, dst.encode(), src.encode());
    }

    pub fn emit_add_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x00, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_sub_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x05, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_and_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x04, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_or_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x01, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_xor_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x06, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_cmp_ri(&mut self, dst: Reg, imm: i32) {
        let opcode = if dst.is_8bit() { 0x80 } else { 0x81 };

        if dst.is_8bit() {
            self.emit_rex(false, false, false, false);
        } else {
            self.emit_rex(dst.is_64bit(), false, false, false);
        }

        self.code.push(opcode);
        self.emit_modrm(0x03, 0x07, dst.encode());

        if dst.is_64bit() {
            self.code
                .extend_from_slice(&(imm as i64).to_le_bytes()[..4]);
        } else {
            self.code.extend_from_slice(&imm.to_le_bytes());
        }
    }

    pub fn emit_lea(&mut self, dst: Reg, addr: &Address) {
        self.emit_rex(dst.is_64bit(), false, dst.is_64bit(), false);
        self.code.push(0x8D);
        addr.emit_modrm(&mut self.code, dst.encode());

        if let Some(disp) = addr.displacement {
            self.code.extend_from_slice(&disp.to_le_bytes());
        }
    }

    pub fn emit_sub_rsp(&mut self, imm: u8) {
        if imm == 0 {
            return;
        }
        self.emit_rex(false, false, false, false);
        if imm <= 127 {
            self.code.push(0x48);
            self.code.push(0x83);
            self.code.push(0xEC);
            self.code.push(imm);
        } else {
            self.code.push(0x48);
            self.code.push(0x81);
            self.code.push(0xEC);
            self.code.extend_from_slice(&(imm as u32).to_le_bytes());
        }
    }

    pub fn emit_add_rsp(&mut self, imm: u8) {
        if imm == 0 {
            return;
        }
        self.emit_rex(false, false, false, false);
        if imm <= 127 {
            self.code.push(0x48);
            self.code.push(0x83);
            self.code.push(0xC4);
            self.code.push(imm);
        } else {
            self.code.push(0x48);
            self.code.push(0x81);
            self.code.push(0xC4);
            self.code.extend_from_slice(&(imm as u32).to_le_bytes());
        }
    }

    pub fn emit_nop(&mut self) {
        self.code.push(0x90);
    }

    pub fn emit_int3(&mut self) {
        self.code.push(0xCC);
    }

    pub fn emit_syscall(&mut self) {
        self.emit_rex(true, false, false, false);
        self.code.push(0x0F);
        self.code.push(0x05);
    }

    pub fn emit_div_i64(&mut self, divisor: Reg) {
        self.emit_rex(true, false, false, divisor.is_64bit());
        self.code.push(0xF7);
        self.emit_modrm(0x03, 0x07, divisor.encode());
    }

    pub fn emit_div_u64(&mut self, divisor: Reg) {
        self.emit_rex(false, false, false, divisor.is_64bit());
        self.code.push(0xF7);
        self.emit_modrm(0x03, 0x06, divisor.encode());
    }

    pub fn emit_div_i32(&mut self, divisor: Reg) {
        self.code.push(0xF7);
        self.emit_modrm(0x03, divisor.encode(), 0x07);
    }

    pub fn emit_div_u32(&mut self, divisor: Reg) {
        self.code.push(0xF7);
        self.emit_modrm(0x03, divisor.encode(), 0x06);
    }

    pub fn emit_shl_cl(&mut self, dst: Reg) {
        self.emit_mov_rr(Reg::Rcx, Reg::Rcx);
        self.code.push(0xD3);
        self.emit_modrm(0x03, 0x04, dst.encode());
    }

    pub fn emit_shr_cl(&mut self, dst: Reg) {
        self.emit_mov_rr(Reg::Rcx, Reg::Rcx);
        self.code.push(0xD3);
        self.emit_modrm(0x03, 0x05, dst.encode());
    }

    pub fn emit_sar_cl(&mut self, dst: Reg) {
        self.emit_mov_rr(Reg::Rcx, Reg::Rcx);
        self.code.push(0xD3);
        self.emit_modrm(0x03, 0x07, dst.encode());
    }

    pub fn emit_mov_ri32(&mut self, dst: Reg, imm: u32) {
        self.code.push(0xB8 | dst.encode());
        self.code.extend_from_slice(&imm.to_le_bytes());
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emitter_creation() {
        let emitter = Emitter::new();
        assert!(emitter.code().is_empty());
    }

    #[test]
    fn test_mov_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_mov_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_mov_ra() {
        let mut emitter = Emitter::new();
        emitter.emit_mov_ra(Reg::Rax, 42);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_add_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_add_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_sub_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_sub_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_mul_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_mul_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_jmp_rel32() {
        let mut emitter = Emitter::new();
        emitter.emit_jmp_rel32(100);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_jcc_rel32() {
        let mut emitter = Emitter::new();
        emitter.emit_jcc_rel32(Condition::Equal, 50);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_call_rel32() {
        let mut emitter = Emitter::new();
        emitter.emit_call_rel32(1000);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_ret() {
        let mut emitter = Emitter::new();
        emitter.emit_ret();
        assert_eq!(emitter.code(), &[0xC3]);
    }

    #[test]
    fn test_pop() {
        let mut emitter = Emitter::new();
        emitter.emit_pop(Reg::Rax);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_push() {
        let mut emitter = Emitter::new();
        emitter.emit_push(Reg::Rax);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_cmp_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_cmp_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_test_rr() {
        let mut emitter = Emitter::new();
        emitter.emit_test_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_shift_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_shl_ri(Reg::Rax, 4);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_shr_ri(Reg::Rax, 2);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_sar_ri(Reg::Rax, 1);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_rotate_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_rol_ri(Reg::Rax, 4);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_ror_ri(Reg::Rax, 4);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_logical_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_xor_rr(Reg::Rax, Reg::Rax);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_or_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_and_rr(Reg::Rax, Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_immediate_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_add_ri(Reg::Rax, 100);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_sub_ri(Reg::Rax, 50);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_cmp_ri(Reg::Rax, 10);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_stack_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_sub_rsp(32);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_add_rsp(32);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_div_operations() {
        let mut emitter = Emitter::new();
        emitter.emit_div_i64(Reg::Rcx);
        assert!(!emitter.code().is_empty());

        let mut emitter = Emitter::new();
        emitter.emit_div_u64(Reg::Rcx);
        assert!(!emitter.code().is_empty());
    }

    #[test]
    fn test_byte_emission() {
        let mut emitter = Emitter::new();
        emitter.emit_byte(0x90);
        assert_eq!(emitter.code(), &[0x90]);
    }

    #[test]
    fn test_mov_rr_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_mov_rr(Reg::Rax, Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 3);
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0x89);
        assert_eq!(code[2], 0xc8);
    }

    #[test]
    fn test_add_rr_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_add_rr(Reg::Rax, Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 3);
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0x01);
        assert_eq!(code[2], 0xc8);
    }

    #[test]
    fn test_sub_rr_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_sub_rr(Reg::Rax, Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 3);
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0x29);
        assert_eq!(code[2], 0xc8);
    }

    #[test]
    fn test_cmp_rr_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_cmp_rr(Reg::Rax, Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 3);
        assert_eq!(code[0], 0x48);
        assert_eq!(code[1], 0x39);
        assert_eq!(code[2], 0xc8);
    }

    #[test]
    fn test_ret_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_ret();
        let code = emitter.code();
        assert_eq!(code, &[0xC3]);
    }

    #[test]
    fn test_push_pop_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_push(Reg::Rax);
        let code = emitter.code();
        assert_eq!(code, &[0x48, 0x50]);

        let mut emitter = Emitter::new();
        emitter.emit_pop(Reg::Rax);
        let code = emitter.code();
        assert_eq!(code, &[0x48, 0x58]);
    }

    #[test]
    fn test_cdq_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_cdq();
        let code = emitter.code();
        assert_eq!(code, &[0x99]);
    }

    #[test]
    fn test_jmp_rel32_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_jmp_rel32(0x00001000);
        let code = emitter.code();
        assert_eq!(code.len(), 5);
        assert_eq!(code[0], 0xE9);
        assert_eq!(code[1..5], 0x00001000u32.to_le_bytes());
    }

    #[test]
    fn test_jcc_rel32_encoding() {
        use super::Condition;
        let mut emitter = Emitter::new();
        emitter.emit_jcc_rel32(Condition::Equal, 0x00001000);
        let code = emitter.code();
        assert_eq!(code.len(), 6);
        assert_eq!(code[0], 0x0F);
        assert_eq!(code[1], 0x80 | Condition::Equal.encode());
        assert_eq!(code[2..6], 0x00001000u32.to_le_bytes());
    }

    #[test]
    fn test_mov_ri32_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_mov_ri32(Reg::Rax, 0x12345678);
        let code = emitter.code();
        assert_eq!(code.len(), 5);
        assert_eq!(code[0], 0xB8 | Reg::Rax.encode());
        assert_eq!(code[1..5], 0x12345678u32.to_le_bytes());
    }

    #[test]
    fn test_cmp_ri_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_cmp_ri(Reg::Rax, 42);
        let code = emitter.code();
        assert!(!code.is_empty());
        assert_eq!(code[code.len() - 4..], 42i32.to_le_bytes());
    }

    #[test]
    fn test_div_i32_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_div_i32(Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 2);
        assert_eq!(code[0], 0xF7);
        assert_eq!(code[1], (0x03 << 6) | (0x07 << 3) | Reg::Rcx.encode());
    }

    #[test]
    fn test_div_u32_encoding() {
        let mut emitter = Emitter::new();
        emitter.emit_div_u32(Reg::Rcx);
        let code = emitter.code();
        assert_eq!(code.len(), 2);
        assert_eq!(code[0], 0xF7);
        assert_eq!(code[1], (0x03 << 6) | (0x06 << 3) | Reg::Rcx.encode());
    }
}
