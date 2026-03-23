use super::BinaryReader;
use crate::runtime::{
    DataKind, DataSegment, ElemKind, ElemSegment, ExportKind, ExportType, Import, ImportKind,
};
use crate::runtime::{Func, Local, Result, WasmError};
use crate::runtime::{
    FunctionType, GlobalType, Limits, MemoryType, Module, NumType, RefType, TableType, ValType,
};
use std::io::Cursor;

const MAGIC: u32 = 0x6D736100;
const CURRENT_VERSION: u32 = 1;

#[derive(Debug)]
pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, data: &[u8]) -> Result<Module> {
        let mut reader = BinaryReader::from_slice(data);
        self.parse_module(&mut reader)
    }

    fn parse_module(&self, reader: &mut BinaryReader<std::io::Cursor<&[u8]>>) -> Result<Module> {
        let magic = reader
            .read_u32()
            .map_err(|_| WasmError::Load("invalid magic number".to_string()))?;
        if magic != MAGIC {
            return Err(WasmError::Load(format!(
                "invalid magic number: {:x}",
                magic
            )));
        }

        let version = reader
            .read_u32()
            .map_err(|_| WasmError::Load("invalid version".to_string()))?;
        if version != CURRENT_VERSION {
            return Err(WasmError::Load(format!("unsupported version: {}", version)));
        }

        let mut module = Module::new();

        while reader.remaining() > 0 {
            let section_id = reader
                .read_u8()
                .map_err(|_| WasmError::Load("failed to read section id".to_string()))?;
            let _section_size = reader
                .read_u32()
                .map_err(|_| WasmError::Load("failed to read section size".to_string()))?;

            match section_id {
                0 => self.parse_custom(reader)?,
                1 => self.parse_type(reader, &mut module)?,
                2 => self.parse_import(reader, &mut module)?,
                3 => self.parse_function(reader, &mut module)?,
                4 => self.parse_table(reader, &mut module)?,
                5 => self.parse_memory(reader, &mut module)?,
                6 => self.parse_global(reader, &mut module)?,
                7 => self.parse_export(reader, &mut module)?,
                8 => self.parse_start(reader, &mut module)?,
                9 => self.parse_elem(reader, &mut module)?,
                10 => self.parse_code(reader, &mut module)?,
                11 => self.parse_data(reader, &mut module)?,
                _ => {
                    return Err(WasmError::Load(format!(
                        "unknown section id: {}",
                        section_id
                    )))
                }
            }
        }

        Ok(module)
    }

    fn parse_custom(&self, reader: &mut BinaryReader<Cursor<&[u8]>>) -> Result<()> {
        let _name_len = reader.read_uleb128()?;
        loop {
            if reader.remaining() == 0 {
                break;
            }
            match reader.read_u8() {
                Ok(_) => {}
                Err(_) => break,
            }
        }
        Ok(())
    }

    fn parse_type(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let form = reader.read_i32()?;
            if form != -0x01 {
                return Err(WasmError::Load("expected func type form".to_string()));
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
            module.types.push(FunctionType::new(params, results));
        }
        Ok(())
    }

    fn read_val_type(&self, reader: &mut BinaryReader<Cursor<&[u8]>>) -> Result<ValType> {
        let type_byte = reader.read_u8()?;
        match type_byte {
            0x7F => Ok(ValType::Num(NumType::I32)),
            0x7E => Ok(ValType::Num(NumType::I64)),
            0x7D => Ok(ValType::Num(NumType::F32)),
            0x7C => Ok(ValType::Num(NumType::F64)),
            0x70 => Ok(ValType::Ref(RefType::FuncRef)),
            0x6F => Ok(ValType::Ref(RefType::ExternRef)),
            _ => Err(WasmError::Load(format!("unknown val type: {}", type_byte))),
        }
    }

    fn parse_import(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
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
                0x00 => {
                    let type_idx = reader.read_uleb128()?;
                    ImportKind::Func(type_idx)
                }
                0x01 => {
                    let elem_type = reader.read_u8()?;
                    let limits = self.read_limits(reader)?;
                    if elem_type != 0x70 {
                        return Err(WasmError::Load("expected funcref table".to_string()));
                    }
                    ImportKind::Table(TableType::new(RefType::FuncRef, limits))
                }
                0x02 => {
                    let limits = self.read_limits(reader)?;
                    ImportKind::Memory(MemoryType::new(limits))
                }
                0x03 => {
                    let content_type = self.read_val_type(reader)?;
                    let mutable = reader.read_u8()? != 0;
                    ImportKind::Global(GlobalType::new(content_type, mutable))
                }
                _ => {
                    return Err(WasmError::Load(format!(
                        "unknown import kind: {}",
                        kind_byte
                    )))
                }
            };

            module.imports.push(Import {
                module: module_str,
                name: name_str,
                kind,
            });
        }
        Ok(())
    }

    fn parse_function(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let type_idx = reader.read_uleb128()?;
            module.funcs.push(Func {
                type_idx,
                locals: Vec::new(),
                body: Vec::new(),
            });
        }
        Ok(())
    }

    fn parse_table(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let elem_type = reader.read_u8()?;
            if elem_type != 0x70 && elem_type != 0x6F {
                return Err(WasmError::Load(
                    "expected funcref or externref table".to_string(),
                ));
            }
            let limits = self.read_limits(reader)?;
            let ref_type = if elem_type == 0x70 {
                RefType::FuncRef
            } else {
                RefType::ExternRef
            };
            module.tables.push(TableType::new(ref_type, limits));
        }
        Ok(())
    }

    fn parse_memory(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let limits = self.read_limits(reader)?;
            module.memories.push(MemoryType::new(limits));
        }
        Ok(())
    }

    fn read_limits(&self, reader: &mut BinaryReader<Cursor<&[u8]>>) -> Result<Limits> {
        let flags = reader.read_u8()?;
        if flags & 0x01 == 0 {
            let min = reader.read_uleb128()?;
            Ok(Limits::Min(min))
        } else {
            let min = reader.read_uleb128()?;
            let max = reader.read_uleb128()?;
            Ok(Limits::MinMax(min, max))
        }
    }

    fn parse_global(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let content_type = self.read_val_type(reader)?;
            let mutable = reader.read_u8()? != 0;
            let _init = reader.read_bytes(10)?;
            module.globals.push(GlobalType::new(content_type, mutable));
        }
        Ok(())
    }

    fn parse_export(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
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

            module.exports.push(ExportType {
                name: name_str,
                kind,
            });
        }
        Ok(())
    }

    fn parse_start(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let func_idx = reader.read_uleb128()?;
        module.start = Some(func_idx);
        Ok(())
    }

    fn parse_elem(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let kind = reader.read_u8()?;
            let (table_idx, offset) = if kind == 0x00 {
                let offset = reader.read_bytes(10)?;
                (0, offset)
            } else {
                let table_idx = reader.read_uleb128()?;
                let offset = reader.read_bytes(10)?;
                (table_idx, offset)
            };
            let num_elem = reader.read_uleb128()?;
            let mut elems = Vec::new();
            for _ in 0..num_elem {
                let func_idx = reader.read_uleb128()?;
                elems.push(func_idx);
            }
            module.elems.push(ElemSegment {
                kind: ElemKind::Active { table_idx, offset },
                type_: RefType::FuncRef,
            });
        }
        Ok(())
    }

    fn parse_code(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        if count as usize != module.funcs.len() {
            return Err(WasmError::Load(
                "function and code section size mismatch".to_string(),
            ));
        }
        for func in &mut module.funcs {
            let body_size = reader.read_uleb128()?;
            let locals = self.parse_locals(reader)?;
            func.locals = locals;
            func.body =
                reader.read_bytes(body_size as usize - self.calc_locals_size(&func.locals))?;
        }
        Ok(())
    }

    fn parse_locals(&self, reader: &mut BinaryReader<Cursor<&[u8]>>) -> Result<Vec<Local>> {
        let count = reader.read_uleb128()?;
        let mut locals = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let n = reader.read_uleb128()?;
            let type_ = self.read_val_type(reader)?;
            locals.push(Local { count: n, type_ });
        }
        Ok(locals)
    }

    fn calc_locals_size(&self, locals: &[Local]) -> usize {
        let mut size = 0;
        for local in locals {
            let type_size = match local.type_ {
                ValType::Num(NumType::I32) | ValType::Num(NumType::F32) | ValType::Ref(_) => 1,
                ValType::Num(NumType::I64) | ValType::Num(NumType::F64) => 2,
            };
            size += type_size * local.count as usize;
        }
        size
    }

    fn parse_data(
        &self,
        reader: &mut BinaryReader<Cursor<&[u8]>>,
        module: &mut Module,
    ) -> Result<()> {
        let count = reader.read_uleb128()?;
        for _ in 0..count {
            let kind = reader.read_u8()?;
            let (memory_idx, offset) = if kind == 0x00 {
                let offset = reader.read_bytes(10)?;
                (0, offset)
            } else {
                (0, Vec::new())
            };
            let init_size = reader.read_uleb128()? as usize;
            let init = reader.read_bytes(init_size)?;
            module.data.push(DataSegment {
                kind: DataKind::Active { memory_idx, offset },
                init,
            });
        }
        Ok(())
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_module() {
        let data = vec![0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let parser = Parser::new();
        let module = parser.parse(&data).unwrap();
        assert_eq!(module.types.len(), 0);
        assert_eq!(module.funcs.len(), 0);
    }
}
