use crate::runtime::ValType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    NullRef,
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
            WasmValue::NullRef => ValType::Ref(crate::runtime::RefType::ExternRef),
            WasmValue::FuncRef(_) => ValType::Ref(crate::runtime::RefType::FuncRef),
            WasmValue::ExternRef(_) => ValType::Ref(crate::runtime::RefType::ExternRef),
        }
    }

    pub fn i32(&self) -> i32 {
        match self {
            WasmValue::I32(v) => *v,
            _ => panic!("expected i32"),
        }
    }

    pub fn i64(&self) -> i64 {
        match self {
            WasmValue::I64(v) => *v,
            _ => panic!("expected i64"),
        }
    }

    pub fn f32(&self) -> f32 {
        match self {
            WasmValue::F32(v) => *v,
            _ => panic!("expected f32"),
        }
    }

    pub fn f64(&self) -> f64 {
        match self {
            WasmValue::F64(v) => *v,
            _ => panic!("expected f64"),
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
        assert_eq!(val.i32(), 42);
    }
}
