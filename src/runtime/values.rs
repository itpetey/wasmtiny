use crate::runtime::{RefType, Result, ValType, WasmError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    NullRef(RefType),
    FuncRef(u32),
    ExternRef(u32),
}

impl WasmValue {
    pub fn val_type(&self) -> ValType {
        match self {
            WasmValue::I32(_) => ValType::Num(crate::runtime::NumType::I32),
            WasmValue::I64(_) => ValType::Num(crate::runtime::NumType::I64),
            WasmValue::F32(_) => ValType::Num(crate::runtime::NumType::F32),
            WasmValue::F64(_) => ValType::Num(crate::runtime::NumType::F64),
            WasmValue::NullRef(ref_type) => ValType::Ref(*ref_type),
            WasmValue::FuncRef(_) => ValType::Ref(crate::runtime::RefType::FuncRef),
            WasmValue::ExternRef(_) => ValType::Ref(crate::runtime::RefType::ExternRef),
        }
    }

    pub fn i32(&self) -> Result<i32> {
        match self {
            WasmValue::I32(v) => Ok(*v),
            _ => Err(WasmError::Runtime("expected i32".to_string())),
        }
    }

    pub fn i64(&self) -> Result<i64> {
        match self {
            WasmValue::I64(v) => Ok(*v),
            _ => Err(WasmError::Runtime("expected i64".to_string())),
        }
    }

    pub fn f32(&self) -> Result<f32> {
        match self {
            WasmValue::F32(v) => Ok(*v),
            _ => Err(WasmError::Runtime("expected f32".to_string())),
        }
    }

    pub fn f64(&self) -> Result<f64> {
        match self {
            WasmValue::F64(v) => Ok(*v),
            _ => Err(WasmError::Runtime("expected f64".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_val_type() {
        assert_eq!(
            WasmValue::I32(42).val_type(),
            ValType::Num(crate::runtime::NumType::I32)
        );
        assert_eq!(
            WasmValue::F64(3.14).val_type(),
            ValType::Num(crate::runtime::NumType::F64)
        );
    }

    #[test]
    fn test_accessors() {
        let val = WasmValue::I32(42);
        assert_eq!(val.i32().unwrap(), 42);
    }
}
