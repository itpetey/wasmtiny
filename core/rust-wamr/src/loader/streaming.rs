use super::BinaryReader;
use crate::runtime::{
    DataKind, DataSegment, ElemKind, ElemSegment, ExportKind, ExportType, Import, ImportKind,
};
use crate::runtime::{Func, Local};
use crate::runtime::{FunctionType, Module, NumType, RefType, Result, ValType, WasmError};
use crate::runtime::{GlobalType, Limits, MemoryType, TableType};
use std::io::Cursor;

pub struct StreamingParser {
    pub module: Module,
    section_count: usize,
}

impl StreamingParser {
    pub fn new() -> Self {
        Self {
            module: Module::new(),
            section_count: 0,
        }
    }

    pub fn parse_chunk(&mut self, data: &[u8]) -> Result<ParseState> {
        let mut reader = BinaryReader::from_slice(data);

        if self.module.types.is_empty() && self.module.funcs.is_empty() && self.section_count == 0 {
            let magic = reader
                .read_u32()
                .map_err(|_| WasmError::Load("invalid magic number".to_string()))?;
            if magic != 0x6D736100 {
                return Err(WasmError::Load(format!("invalid magic: {:x}", magic)));
            }
            let version = reader
                .read_u32()
                .map_err(|_| WasmError::Load("invalid version".to_string()))?;
            if version != 1 {
                return Err(WasmError::Load(format!("unsupported version: {}", version)));
            }
        }

        while reader.remaining() > 0 {
            let section_id = reader
                .read_u8()
                .map_err(|_| WasmError::Load("failed to read section id".to_string()))?;
            let section_size = reader
                .read_u32()
                .map_err(|_| WasmError::Load("failed to read section size".to_string()))?;

            self.section_count += 1;

            match section_id {
                0 => {}
                1 => self.parse_types(&mut reader)?,
                2 => self.parse_imports(&mut reader)?,
                3 => self.parse_functions(&mut reader)?,
                4 => self.parse_tables(&mut reader)?,
                5 => self.parse_memories(&mut reader)?,
                6 => self.parse_globals(&mut reader)?,
                7 => self.parse_exports(&mut reader)?,
                8 => self.parse_start(&mut reader)?,
                9 => self.parse_elems(&mut reader)?,
                10 => self.parse_code(&mut reader)?,
                11 => self.parse_data(&mut reader)?,
                _ => return Err(WasmError::Load(format!("unknown section: {}", section_id))),
            }
        }

        if reader.remaining() == 0 && self.section_count > 0 {
            Ok(ParseState::Complete)
        } else if reader.remaining() > 0 {
            Ok(ParseState::NeedMoreData)
        } else {
            Ok(ParseState::Complete)
        }
    }

    fn parse_types(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let form = reader.read_i32()?;
            if form != -0x01 {
                return Err(WasmError::Load("expected func type".to_string()));
            }
            let param_count = reader.read_uleb128()?;
            let mut params = Vec::with_capacity(param_count as usize);
            for _ in 0..param_count {
                params.push(self.read_val_type(reader)?);
            }
            let result_count = reader.read_uleb128()?;
            let mut results = Vec::with_capacity(result_count as usize);
            for _ in 0..result_count {
                results.push(self.read_val_type(reader)?);
            }
            self.module.types.push(FunctionType::new(params, results));
        }
        Ok(())
    }

    fn read_val_type(&self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<ValType> {
        let byte = reader.read_u8()?;
        match byte {
            0x7F => Ok(ValType::Num(NumType::I32)),
            0x7E => Ok(ValType::Num(NumType::I64)),
            0x7D => Ok(ValType::Num(NumType::F32)),
            0x7C => Ok(ValType::Num(NumType::F64)),
            0x70 => Ok(ValType::Ref(RefType::FuncRef)),
            0x6F => Ok(ValType::Ref(RefType::ExternRef)),
            _ => Err(WasmError::Load(format!("unknown val type: {}", byte))),
        }
    }

    fn parse_imports(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let module_len = reader.read_uleb128()?;
            let module_name = reader.read_bytes(module_len as usize)?;
            let module_str = String::from_utf8_lossy(&module_name).to_string();
            let name_len = reader.read_uleb128()?;
            let name = reader.read_bytes(name_len as usize)?;
            let name_str = String::from_utf8_lossy(&name).to_string();
            let kind_byte = reader.read_u8()?;
            let kind = match kind_byte {
                0x00 => ImportKind::Func(reader.read_uleb128()?),
                0x01 => ImportKind::Table(TableType::new(
                    RefType::FuncRef,
                    Limits::Min(reader.read_uleb128()?),
                )),
                0x02 => ImportKind::Memory(MemoryType::new(Limits::Min(reader.read_uleb128()?))),
                0x03 => ImportKind::Global(GlobalType::new(ValType::Num(NumType::I32), false)),
                _ => {
                    return Err(WasmError::Load(format!(
                        "unknown import kind: {}",
                        kind_byte
                    )))
                }
            };
            self.module.imports.push(Import {
                module: module_str,
                name: name_str,
                kind,
            });
        }
        Ok(())
    }

    fn parse_functions(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            self.module.funcs.push(Func {
                type_idx: reader.read_uleb128()?,
                locals: vec![],
                body: vec![],
            });
        }
        Ok(())
    }

    fn parse_tables(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let elem_type = reader.read_u8()?;
            if elem_type != 0x70 && elem_type != 0x6F {
                return Err(WasmError::Load(
                    "expected funcref/externref table".to_string(),
                ));
            }
            let min = reader.read_uleb128()?;
            let max = if min > 0x10000000 {
                Some(reader.read_uleb128()?)
            } else {
                None
            };
            let limits = match max {
                Some(m) => Limits::MinMax(min, m),
                None => Limits::Min(min),
            };
            self.module
                .tables
                .push(TableType::new(RefType::FuncRef, limits));
        }
        Ok(())
    }

    fn parse_memories(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let min = reader.read_uleb128()?;
            let max = if min > 16 {
                Some(reader.read_uleb128()?)
            } else {
                None
            };
            let limits = match max {
                Some(m) => Limits::MinMax(min, m),
                None => Limits::Min(min),
            };
            self.module.memories.push(MemoryType::new(limits));
        }
        Ok(())
    }

    fn parse_globals(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let content_type = self.read_val_type(reader)?;
            let mutable = reader.read_u8()? != 0;
            let _init = reader.read_bytes(10)?;
            self.module
                .globals
                .push(GlobalType::new(content_type, mutable));
        }
        Ok(())
    }

    fn parse_exports(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let name_len = reader.read_uleb128()?;
            let name = reader.read_bytes(name_len as usize)?;
            let name_str = String::from_utf8_lossy(&name).to_string();
            let kind_byte = reader.read_u8()?;
            let idx = reader.read_uleb128()?;
            let kind = match kind_byte {
                0x00 => ExportKind::Func(idx),
                0x01 => ExportKind::Table(idx),
                0x02 => ExportKind::Memory(idx),
                0x03 => ExportKind::Global(idx),
                _ => {
                    return Err(WasmError::Load(format!(
                        "unknown export kind: {}",
                        kind_byte
                    )))
                }
            };
            self.module.exports.push(ExportType {
                name: name_str,
                kind,
            });
        }
        Ok(())
    }

    fn parse_start(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        self.module.start = Some(reader.read_uleb128()?);
        Ok(())
    }

    fn parse_elems(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let kind = reader.read_u8()?;
            let (table_idx, offset) = if kind == 0x00 {
                let offset = reader.read_bytes(10)?;
                (0, offset)
            } else {
                (reader.read_uleb128()?, vec![])
            };
            let num_elem = reader.read_uleb128()?;
            for _ in 0..num_elem {
                let _func_idx = reader.read_uleb128()?;
            }
            self.module.elems.push(ElemSegment {
                kind: ElemKind::Active { table_idx, offset },
                type_: RefType::FuncRef,
            });
        }
        Ok(())
    }

    fn parse_code(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        if count as usize != self.module.funcs.len() {
            return Err(WasmError::Load("function/code count mismatch".to_string()));
        }

        let mut all_locals: Vec<Vec<(u32, ValType)>> = Vec::new();
        for _ in 0..self.module.funcs.len() {
            let locals_count = reader.read_uleb128()?;
            let mut locals = Vec::new();
            for _ in 0..locals_count {
                let n = reader.read_uleb128()?;
                let type_val = reader.read_byte()?;
                let type_ = match type_val {
                    0x7F => ValType::Num(NumType::I32),
                    0x7E => ValType::Num(NumType::I64),
                    0x7D => ValType::Num(NumType::F32),
                    0x7C => ValType::Num(NumType::F64),
                    0x70 => ValType::Ref(RefType::FuncRef),
                    0x6F => ValType::Ref(RefType::ExternRef),
                    _ => ValType::Num(NumType::I32),
                };
                locals.push((n, type_));
            }
            all_locals.push(locals);
        }

        for (i, func) in self.module.funcs.iter_mut().enumerate() {
            func.locals = all_locals[i]
                .iter()
                .map(|(n, t)| Local {
                    count: *n,
                    type_: t.clone(),
                })
                .collect();
            let body_size = 100;
            func.body = reader.read_bytes(body_size)?;
        }
        Ok(())
    }

    fn parse_data(&mut self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let kind = reader.read_u8()?;
            let (memory_idx, offset) = if kind == 0x00 {
                let offset = reader.read_bytes(10)?;
                (0, offset)
            } else {
                (0, vec![])
            };
            let init_len = reader.read_uleb128()? as usize;
            let init = reader.read_bytes(init_len)?;
            self.module.data.push(DataSegment {
                kind: DataKind::Active { memory_idx, offset },
                init,
            });
        }
        Ok(())
    }

    pub fn into_module(self) -> Module {
        self.module
    }
}

impl Default for StreamingParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseState {
    Complete,
    NeedMoreData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_parser() {
        let data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let mut parser = StreamingParser::new();
        let state = parser.parse_chunk(&data).unwrap();
        assert_eq!(state, ParseState::Complete);
    }
}
