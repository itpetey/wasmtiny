#[derive(Debug, Clone, PartialEq)]
/// Decoded WebAssembly instruction.
pub enum Instruction {
    /// WebAssembly `unreachable` instruction.
    Unreachable,
    /// WebAssembly `nop` instruction.
    Nop,
    /// WebAssembly `block` instruction.
    Block(BlockType),
    /// WebAssembly `loop` instruction.
    Loop(BlockType),
    /// WebAssembly `if` instruction.
    If(BlockType),
    /// WebAssembly `else` instruction.
    Else,
    /// WebAssembly `end` instruction.
    End,
    /// WebAssembly `br` instruction.
    Br(u32),
    /// WebAssembly `br.if` instruction.
    BrIf(u32),
    /// WebAssembly `br.table` instruction.
    BrTable(Vec<u32>, u32),
    /// WebAssembly `return` instruction.
    Return,
    /// WebAssembly `call` instruction.
    Call(u32),
    /// WebAssembly `call.indirect` instruction.
    CallIndirect(u32, u32),
    /// WebAssembly `drop` instruction.
    Drop,
    /// WebAssembly `select` instruction.
    Select,
    /// WebAssembly `local.get` instruction.
    LocalGet(u32),
    /// WebAssembly `local_set` instruction.
    LocalSet(u32),
    /// WebAssembly `local.tee` instruction.
    LocalTee(u32),
    /// WebAssembly `global.get` instruction.
    GlobalGet(u32),
    /// WebAssembly `global_set` instruction.
    GlobalSet(u32),
    /// WebAssembly `table.get` instruction.
    TableGet(u32),
    /// WebAssembly `table_set` instruction.
    TableSet(u32),
    /// WebAssembly `table_size` instruction.
    TableSize(u32),
    /// WebAssembly `table.grow` instruction.
    TableGrow(u32),
    /// WebAssembly `table.fill` instruction.
    TableFill(u32),
    /// WebAssembly `table.copy` instruction.
    TableCopy(u32, u32),
    /// WebAssembly `table.init` instruction.
    TableInit(u32, u32),
    /// WebAssembly `elem.drop` instruction.
    ElemDrop(u32),
    /// WebAssembly `ref.null` instruction.
    RefNull,
    /// WebAssembly `ref.is_null` instruction.
    RefIsNull,
    /// WebAssembly `ref.func` instruction.
    RefFunc(u32),
    /// WebAssembly `i32.load` instruction.
    I32Load(MemArg),
    /// WebAssembly `i64.load` instruction.
    I64Load(MemArg),
    /// WebAssembly `f32.load` instruction.
    F32Load(MemArg),
    /// WebAssembly `f64.load` instruction.
    F64Load(MemArg),
    /// WebAssembly `i32.load8_s` instruction.
    I32Load8S(MemArg),
    /// WebAssembly `i32.load8_u` instruction.
    I32Load8U(MemArg),
    /// WebAssembly `i32.load16_s` instruction.
    I32Load16S(MemArg),
    /// WebAssembly `i32.load16_u` instruction.
    I32Load16U(MemArg),
    /// WebAssembly `i64.load8_s` instruction.
    I64Load8S(MemArg),
    /// WebAssembly `i64.load8_u` instruction.
    I64Load8U(MemArg),
    /// WebAssembly `i64.load16_s` instruction.
    I64Load16S(MemArg),
    /// WebAssembly `i64.load16_u` instruction.
    I64Load16U(MemArg),
    /// WebAssembly `i64.load32_s` instruction.
    I64Load32S(MemArg),
    /// WebAssembly `i64.load32_u` instruction.
    I64Load32U(MemArg),
    /// WebAssembly `i32_store` instruction.
    I32Store(MemArg),
    /// WebAssembly `i64_store` instruction.
    I64Store(MemArg),
    /// WebAssembly `f32_store` instruction.
    F32Store(MemArg),
    /// WebAssembly `f64_store` instruction.
    F64Store(MemArg),
    /// WebAssembly `i32_store8` instruction.
    I32Store8(MemArg),
    /// WebAssembly `i32_store16` instruction.
    I32Store16(MemArg),
    /// WebAssembly `i64_store8` instruction.
    I64Store8(MemArg),
    /// WebAssembly `i64_store16` instruction.
    I64Store16(MemArg),
    /// WebAssembly `i64_store32` instruction.
    I64Store32(MemArg),
    /// WebAssembly `memory_size` instruction.
    MemorySize,
    /// WebAssembly `memory.grow` instruction.
    MemoryGrow,
    /// WebAssembly `memory.fill` instruction.
    MemoryFill(u32),
    /// WebAssembly `memory.copy` instruction.
    MemoryCopy(u32, u32),
    /// WebAssembly `memory.init` instruction.
    MemoryInit(u32),
    /// WebAssembly `data.drop` instruction.
    DataDrop(u32),
    /// WebAssembly `i32.const` instruction.
    I32Const(i32),
    /// WebAssembly `i64.const` instruction.
    I64Const(i64),
    /// WebAssembly `f32.const` instruction.
    F32Const(f32),
    /// WebAssembly `f64.const` instruction.
    F64Const(f64),
    /// WebAssembly `i32.eqz` instruction.
    I32Eqz,
    /// WebAssembly `i32.eq` instruction.
    I32Eq,
    /// WebAssembly `i32.ne` instruction.
    I32Ne,
    /// WebAssembly `i32.lt_s` instruction.
    I32LtS,
    /// WebAssembly `i32.lt_u` instruction.
    I32LtU,
    /// WebAssembly `i32.gt_s` instruction.
    I32GtS,
    /// WebAssembly `i32.gt_u` instruction.
    I32GtU,
    /// WebAssembly `i32.le_s` instruction.
    I32LeS,
    /// WebAssembly `i32.le_u` instruction.
    I32LeU,
    /// WebAssembly `i32.ge_s` instruction.
    I32GeS,
    /// WebAssembly `i32.ge_u` instruction.
    I32GeU,
    /// WebAssembly `i64.eqz` instruction.
    I64Eqz,
    /// WebAssembly `i64.eq` instruction.
    I64Eq,
    /// WebAssembly `i64.ne` instruction.
    I64Ne,
    /// WebAssembly `i64.lt_s` instruction.
    I64LtS,
    /// WebAssembly `i64.lt_u` instruction.
    I64LtU,
    /// WebAssembly `i64.gt_s` instruction.
    I64GtS,
    /// WebAssembly `i64.gt_u` instruction.
    I64GtU,
    /// WebAssembly `i64.le_s` instruction.
    I64LeS,
    /// WebAssembly `i64.le_u` instruction.
    I64LeU,
    /// WebAssembly `i64.ge_s` instruction.
    I64GeS,
    /// WebAssembly `i64.ge_u` instruction.
    I64GeU,
    /// WebAssembly `f32.eq` instruction.
    F32Eq,
    /// WebAssembly `f32.ne` instruction.
    F32Ne,
    /// WebAssembly `f32.lt` instruction.
    F32Lt,
    /// WebAssembly `f32.gt` instruction.
    F32Gt,
    /// WebAssembly `f32.le` instruction.
    F32Le,
    /// WebAssembly `f32.ge` instruction.
    F32Ge,
    /// WebAssembly `f64.eq` instruction.
    F64Eq,
    /// WebAssembly `f64.ne` instruction.
    F64Ne,
    /// WebAssembly `f64.lt` instruction.
    F64Lt,
    /// WebAssembly `f64.gt` instruction.
    F64Gt,
    /// WebAssembly `f64.le` instruction.
    F64Le,
    /// WebAssembly `f64.ge` instruction.
    F64Ge,
    /// WebAssembly `i32.clz` instruction.
    I32Clz,
    /// WebAssembly `i32.ctz` instruction.
    I32Ctz,
    /// WebAssembly `i32.popcnt` instruction.
    I32Popcnt,
    /// WebAssembly `i32.add` instruction.
    I32Add,
    /// WebAssembly `i32_sub` instruction.
    I32Sub,
    /// WebAssembly `i32.mul` instruction.
    I32Mul,
    /// WebAssembly `i32.div_s` instruction.
    I32DivS,
    /// WebAssembly `i32.div_u` instruction.
    I32DivU,
    /// WebAssembly `i32.rem_s` instruction.
    I32RemS,
    /// WebAssembly `i32.rem_u` instruction.
    I32RemU,
    /// WebAssembly `i32.and` instruction.
    I32And,
    /// WebAssembly `i32.or` instruction.
    I32Or,
    /// WebAssembly `i32.xor` instruction.
    I32Xor,
    /// WebAssembly `i32_shl` instruction.
    I32Shl,
    /// WebAssembly `i32_shr_s` instruction.
    I32ShrS,
    /// WebAssembly `i32_shr_u` instruction.
    I32ShrU,
    /// WebAssembly `i32.rotl` instruction.
    I32Rotl,
    /// WebAssembly `i32.rotr` instruction.
    I32Rotr,
    /// WebAssembly `i64.clz` instruction.
    I64Clz,
    /// WebAssembly `i64.ctz` instruction.
    I64Ctz,
    /// WebAssembly `i64.popcnt` instruction.
    I64Popcnt,
    /// WebAssembly `i64.add` instruction.
    I64Add,
    /// WebAssembly `i64_sub` instruction.
    I64Sub,
    /// WebAssembly `i64.mul` instruction.
    I64Mul,
    /// WebAssembly `i64.div_s` instruction.
    I64DivS,
    /// WebAssembly `i64.div_u` instruction.
    I64DivU,
    /// WebAssembly `i64.rem_s` instruction.
    I64RemS,
    /// WebAssembly `i64.rem_u` instruction.
    I64RemU,
    /// WebAssembly `i64.and` instruction.
    I64And,
    /// WebAssembly `i64.or` instruction.
    I64Or,
    /// WebAssembly `i64.xor` instruction.
    I64Xor,
    /// WebAssembly `i64_shl` instruction.
    I64Shl,
    /// WebAssembly `i64_shr_s` instruction.
    I64ShrS,
    /// WebAssembly `i64_shr_u` instruction.
    I64ShrU,
    /// WebAssembly `i64.rotl` instruction.
    I64Rotl,
    /// WebAssembly `i64.rotr` instruction.
    I64Rotr,
    /// WebAssembly `f32.abs` instruction.
    F32Abs,
    /// WebAssembly `f32.neg` instruction.
    F32Neg,
    /// WebAssembly `f32.ceil` instruction.
    F32Ceil,
    /// WebAssembly `f32.floor` instruction.
    F32Floor,
    /// WebAssembly `f32.trunc` instruction.
    F32Trunc,
    /// WebAssembly `f32.nearest` instruction.
    F32Nearest,
    /// WebAssembly `f32_sqrt` instruction.
    F32Sqrt,
    /// WebAssembly `f32.add` instruction.
    F32Add,
    /// WebAssembly `f32_sub` instruction.
    F32Sub,
    /// WebAssembly `f32.mul` instruction.
    F32Mul,
    /// WebAssembly `f32.div` instruction.
    F32Div,
    /// WebAssembly `f32.min` instruction.
    F32Min,
    /// WebAssembly `f32.max` instruction.
    F32Max,
    /// WebAssembly `f32.copysign` instruction.
    F32Copysign,
    /// WebAssembly `f64.abs` instruction.
    F64Abs,
    /// WebAssembly `f64.neg` instruction.
    F64Neg,
    /// WebAssembly `f64.ceil` instruction.
    F64Ceil,
    /// WebAssembly `f64.floor` instruction.
    F64Floor,
    /// WebAssembly `f64.trunc` instruction.
    F64Trunc,
    /// WebAssembly `f64.nearest` instruction.
    F64Nearest,
    /// WebAssembly `f64_sqrt` instruction.
    F64Sqrt,
    /// WebAssembly `f64.add` instruction.
    F64Add,
    /// WebAssembly `f64_sub` instruction.
    F64Sub,
    /// WebAssembly `f64.mul` instruction.
    F64Mul,
    /// WebAssembly `f64.div` instruction.
    F64Div,
    /// WebAssembly `f64.min` instruction.
    F64Min,
    /// WebAssembly `f64.max` instruction.
    F64Max,
    /// WebAssembly `f64.copysign` instruction.
    F64Copysign,
    /// WebAssembly `i32.wrap.i64` instruction.
    I32WrapI64,
    /// WebAssembly `i32.trunc.f32_s` instruction.
    I32TruncF32S,
    /// WebAssembly `i32.trunc.f32_u` instruction.
    I32TruncF32U,
    /// WebAssembly `i32.trunc.f64_s` instruction.
    I32TruncF64S,
    /// WebAssembly `i32.trunc.f64_u` instruction.
    I32TruncF64U,
    /// WebAssembly `i64.extend.i32_s` instruction.
    I64ExtendI32S,
    /// WebAssembly `i64.extend.i32_u` instruction.
    I64ExtendI32U,
    /// WebAssembly `i64.trunc.f32_s` instruction.
    I64TruncF32S,
    /// WebAssembly `i64.trunc.f32_u` instruction.
    I64TruncF32U,
    /// WebAssembly `i64.trunc.f64_s` instruction.
    I64TruncF64S,
    /// WebAssembly `i64.trunc.f64_u` instruction.
    I64TruncF64U,
    /// WebAssembly `f32.demote.f64` instruction.
    F32DemoteF64,
    /// WebAssembly `f64.promote.f32` instruction.
    F64PromoteF32,
    /// WebAssembly `f32.convert.i32_s` instruction.
    F32ConvertI32S,
    /// WebAssembly `f32.convert.i32_u` instruction.
    F32ConvertI32U,
    /// WebAssembly `f32.convert.i64_s` instruction.
    F32ConvertI64S,
    /// WebAssembly `f32.convert.i64_u` instruction.
    F32ConvertI64U,
    /// WebAssembly `f64.convert.i32_s` instruction.
    F64ConvertI32S,
    /// WebAssembly `f64.convert.i32_u` instruction.
    F64ConvertI32U,
    /// WebAssembly `f64.convert.i64_s` instruction.
    F64ConvertI64S,
    /// WebAssembly `f64.convert.i64_u` instruction.
    F64ConvertI64U,
    /// WebAssembly `i32.reinterpret.f32` instruction.
    I32ReinterpretF32,
    /// WebAssembly `i64.reinterpret.f64` instruction.
    I64ReinterpretF64,
    /// WebAssembly `f32.reinterpret.i32` instruction.
    F32ReinterpretI32,
    /// WebAssembly `f64.reinterpret.i64` instruction.
    F64ReinterpretI64,
    /// WebAssembly `i32.atomic.load` instruction.
    I32AtomicLoad(MemArg),
    /// WebAssembly `i64.atomic.load` instruction.
    I64AtomicLoad(MemArg),
    /// WebAssembly `i32.atomic.load8_u` instruction.
    I32AtomicLoad8U(MemArg),
    /// WebAssembly `i32.atomic.load16_u` instruction.
    I32AtomicLoad16U(MemArg),
    /// WebAssembly `i64.atomic.load8_u` instruction.
    I64AtomicLoad8U(MemArg),
    /// WebAssembly `i64.atomic.load16_u` instruction.
    I64AtomicLoad16U(MemArg),
    /// WebAssembly `i64.atomic.load32_u` instruction.
    I64AtomicLoad32U(MemArg),
    /// WebAssembly `i32.atomic.store` instruction.
    I32AtomicStore(MemArg),
    /// WebAssembly `i64.atomic.store` instruction.
    I64AtomicStore(MemArg),
    /// WebAssembly `i32.atomic.store8` instruction.
    I32AtomicStore8(MemArg),
    /// WebAssembly `i32.atomic.store16` instruction.
    I32AtomicStore16(MemArg),
    /// WebAssembly `i64.atomic.store8` instruction.
    I64AtomicStore8(MemArg),
    /// WebAssembly `i64.atomic.store16` instruction.
    I64AtomicStore16(MemArg),
    /// WebAssembly `i64.atomic.store32` instruction.
    I64AtomicStore32(MemArg),
    /// WebAssembly `i32.atomic.rmw.add` instruction.
    I32AtomicRmwAdd(MemArg),
    /// WebAssembly `i64.atomic.rmw.add` instruction.
    I64AtomicRmwAdd(MemArg),
    /// WebAssembly `i32.atomic.rmw.sub` instruction.
    I32AtomicRmwSub(MemArg),
    /// WebAssembly `i64.atomic.rmw.sub` instruction.
    I64AtomicRmwSub(MemArg),
    /// WebAssembly `i32.atomic.rmw.and` instruction.
    I32AtomicRmwAnd(MemArg),
    /// WebAssembly `i64.atomic.rmw.and` instruction.
    I64AtomicRmwAnd(MemArg),
    /// WebAssembly `i32.atomic.rmw.or` instruction.
    I32AtomicRmwOr(MemArg),
    /// WebAssembly `i64.atomic.rmw.or` instruction.
    I64AtomicRmwOr(MemArg),
    /// WebAssembly `i32.atomic.rmw.xor` instruction.
    I32AtomicRmwXor(MemArg),
    /// WebAssembly `i64.atomic.rmw.xor` instruction.
    I64AtomicRmwXor(MemArg),
    /// WebAssembly `i32.atomic.rmw.xchg` instruction.
    I32AtomicRmwXchg(MemArg),
    /// WebAssembly `i64.atomic.rmw.xchg` instruction.
    I64AtomicRmwXchg(MemArg),
    /// WebAssembly `i32.atomic.rmw.cmpxchg` instruction.
    I32AtomicRmwCmpxchg(MemArg),
    /// WebAssembly `i64.atomic.rmw.cmpxchg` instruction.
    I64AtomicRmwCmpxchg(MemArg),
    /// WebAssembly `memory.atomic.notify` instruction.
    MemoryAtomicNotify(MemArg),
    /// WebAssembly `memory.atomic.wait32` instruction.
    MemoryAtomicWait32(MemArg),
    /// WebAssembly `memory.atomic.wait64` instruction.
    MemoryAtomicWait64(MemArg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Block type.
pub struct BlockType(pub Option<u32>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Mem arg.
pub struct MemArg {
    /// Declared alignment exponent for the memory access.
    pub align: u32,
    /// The offset.
    pub offset: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction() {
        let instr = Instruction::I32Add;
        assert_eq!(instr, Instruction::I32Add);
    }
}
