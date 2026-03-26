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

    pub fn to_bytes(&self, out: &mut Vec<u8>) {
        match self {
            WasmValue::I32(v) => {
                out.push(0);
                out.extend_from_slice(&v.to_le_bytes());
            }
            WasmValue::I64(v) => {
                out.push(1);
                out.extend_from_slice(&v.to_le_bytes());
            }
            WasmValue::F32(v) => {
                out.push(2);
                out.extend_from_slice(&v.to_le_bytes());
            }
            WasmValue::F64(v) => {
                out.push(3);
                out.extend_from_slice(&v.to_le_bytes());
            }
            WasmValue::NullRef(rt) => {
                out.push(4);
                out.push(*rt as u8);
            }
            WasmValue::FuncRef(idx) => {
                out.push(5);
                out.extend_from_slice(&idx.to_le_bytes());
            }
            WasmValue::ExternRef(idx) => {
                out.push(6);
                out.extend_from_slice(&idx.to_le_bytes());
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            0 if bytes.len() >= 5 => {
                let mut v = [0u8; 4];
                v.copy_from_slice(&bytes[1..5]);
                Some((WasmValue::I32(i32::from_le_bytes(v)), 5))
            }
            1 if bytes.len() >= 9 => {
                let mut v = [0u8; 8];
                v.copy_from_slice(&bytes[1..9]);
                Some((WasmValue::I64(i64::from_le_bytes(v)), 9))
            }
            2 if bytes.len() >= 5 => {
                let mut v = [0u8; 4];
                v.copy_from_slice(&bytes[1..5]);
                Some((WasmValue::F32(f32::from_le_bytes(v)), 5))
            }
            3 if bytes.len() >= 9 => {
                let mut v = [0u8; 8];
                v.copy_from_slice(&bytes[1..9]);
                Some((WasmValue::F64(f64::from_le_bytes(v)), 9))
            }
            4 if bytes.len() >= 2 => Some((WasmValue::NullRef(RefType::from_u8(bytes[1])), 2)),
            5 if bytes.len() >= 5 => {
                let mut v = [0u8; 4];
                v.copy_from_slice(&bytes[1..5]);
                Some((WasmValue::FuncRef(u32::from_le_bytes(v)), 5))
            }
            6 if bytes.len() >= 5 => {
                let mut v = [0u8; 4];
                v.copy_from_slice(&bytes[1..5]);
                Some((WasmValue::ExternRef(u32::from_le_bytes(v)), 5))
            }
            _ => None,
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

    #[test]
    fn test_serialization_roundtrip_i32() {
        let val = WasmValue::I32(42);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_i64() {
        let val = WasmValue::I64(12345678901234);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_f32() {
        let val = WasmValue::F32(3.14);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_f64() {
        let val = WasmValue::F64(2.718281828459045);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_nan() {
        let val = WasmValue::F32(f32::NAN);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val.val_type(), restored.val_type());
    }

    #[test]
    fn test_serialization_roundtrip_funcref() {
        let val = WasmValue::FuncRef(42);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_externref() {
        let val = WasmValue::ExternRef(99);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }

    #[test]
    fn test_serialization_roundtrip_nullref() {
        let val = WasmValue::NullRef(crate::runtime::RefType::FuncRef);
        let mut bytes = Vec::new();
        val.to_bytes(&mut bytes);
        let (restored, _) = WasmValue::from_bytes(&bytes).unwrap();
        assert_eq!(val, restored);
    }
}
