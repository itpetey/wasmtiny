use super::Result;
use super::WasmValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefType {
    FuncRef,
    ExternRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValType {
    Num(NumType),
    Ref(RefType),
}

impl ValType {
    pub fn is_numeric(&self) -> bool {
        matches!(self, ValType::Num(_))
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, ValType::Ref(_))
    }

    pub fn as_num_type(&self) -> Option<NumType> {
        match self {
            ValType::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_ref_type(&self) -> Option<RefType> {
        match self {
            ValType::Ref(r) => Some(*r),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub params: Vec<ValType>,
    pub results: Vec<ValType>,
}

impl FunctionType {
    pub fn new(params: Vec<ValType>, results: Vec<ValType>) -> Self {
        Self { params, results }
    }

    pub fn empty() -> Self {
        Self {
            params: Vec::new(),
            results: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limits {
    Min(u32),
    MinMax(u32, u32),
}

impl Limits {
    pub fn min(&self) -> u32 {
        match self {
            Limits::Min(min) => *min,
            Limits::MinMax(min, _) => *min,
        }
    }

    pub fn max(&self) -> Option<u32> {
        match self {
            Limits::Min(_) => None,
            Limits::MinMax(_, max) => Some(*max),
        }
    }

    pub fn matches_required(&self, required: &Limits) -> bool {
        if self.min() < required.min() {
            return false;
        }

        match (self.max(), required.max()) {
            (_, None) => true,
            (Some(actual), Some(required)) => actual <= required,
            (None, Some(_)) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableType {
    pub elem_type: RefType,
    pub limits: Limits,
}

impl TableType {
    pub fn new(elem_type: RefType, limits: Limits) -> Self {
        Self { elem_type, limits }
    }

    pub fn matches_required(&self, required: &TableType) -> bool {
        self.elem_type == required.elem_type && self.limits.matches_required(&required.limits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryType {
    pub limits: Limits,
}

impl MemoryType {
    pub fn new(limits: Limits) -> Self {
        Self { limits }
    }

    pub fn page_size() -> u32 {
        65536
    }

    pub fn matches_required(&self, required: &MemoryType) -> bool {
        self.limits.matches_required(&required.limits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalType {
    pub content_type: ValType,
    pub mutable: bool,
}

impl GlobalType {
    pub fn new(content_type: ValType, mutable: bool) -> Self {
        Self {
            content_type,
            mutable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub type_: TableType,
    pub data: Vec<WasmValue>,
}

impl Table {
    pub fn new(type_: TableType) -> Self {
        let size = type_.limits.min() as usize;
        let default = WasmValue::NullRef(type_.elem_type);
        Self {
            type_,
            data: vec![default; size],
        }
    }

    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }

    pub fn get(&self, idx: u32) -> Option<WasmValue> {
        self.data.get(idx as usize).copied()
    }

    pub fn set(&mut self, idx: u32, val: WasmValue) -> Result<()> {
        if idx as usize >= self.data.len() {
            return Err(super::WasmError::Trap(super::TrapCode::TableOutOfBounds));
        }
        if val.val_type() != ValType::Ref(self.type_.elem_type) {
            return Err(super::WasmError::Validation(
                "table element type mismatch".to_string(),
            ));
        }
        self.data[idx as usize] = val;
        Ok(())
    }

    pub fn grow(&mut self, delta: u32) -> Result<u32> {
        let old_size = self.size();
        let new_size = old_size.saturating_add(delta);
        if let Some(max) = self.type_.limits.max()
            && new_size > max
        {
            return Err(super::WasmError::Runtime(
                "table size exceeds maximum".to_string(),
            ));
        }
        self.data
            .resize(new_size as usize, WasmValue::NullRef(self.type_.elem_type));
        Ok(old_size)
    }
}

#[derive(Debug, Clone)]
pub struct Global {
    pub type_: GlobalType,
    pub value: super::WasmValue,
}

impl Global {
    pub fn new(type_: GlobalType, value: super::WasmValue) -> Result<Self> {
        if type_.content_type != value.val_type() {
            return Err(super::WasmError::Validation(
                "global type mismatch".to_string(),
            ));
        }
        Ok(Self { type_, value })
    }

    pub fn get(&self) -> super::WasmValue {
        self.value
    }

    pub fn set(&mut self, value: super::WasmValue) -> Result<()> {
        if !self.type_.mutable {
            return Err(super::WasmError::Runtime(
                "global is not mutable".to_string(),
            ));
        }
        if self.type_.content_type != value.val_type() {
            return Err(super::WasmError::Validation(
                "value type mismatch".to_string(),
            ));
        }
        self.value = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valtype_is_numeric() {
        assert!(ValType::Num(NumType::I32).is_numeric());
        assert!(ValType::Ref(RefType::FuncRef).is_reference());
    }

    #[test]
    fn test_function_type() {
        let ft = FunctionType::new(
            vec![ValType::Num(NumType::I32), ValType::Num(NumType::I64)],
            vec![ValType::Num(NumType::F32)],
        );
        assert_eq!(ft.params.len(), 2);
        assert_eq!(ft.results.len(), 1);
    }

    #[test]
    fn test_limits() {
        let min = Limits::Min(10);
        assert_eq!(min.min(), 10);
        assert_eq!(min.max(), None);

        let minmax = Limits::MinMax(10, 100);
        assert_eq!(minmax.min(), 10);
        assert_eq!(minmax.max(), Some(100));
    }

    #[test]
    fn test_limits_match_required_subtyping() {
        assert!(Limits::MinMax(2, 3).matches_required(&Limits::MinMax(1, 4)));
        assert!(Limits::MinMax(2, 3).matches_required(&Limits::Min(1)));
        assert!(!Limits::MinMax(1, 5).matches_required(&Limits::MinMax(2, 4)));
        assert!(!Limits::Min(2).matches_required(&Limits::MinMax(1, 4)));
    }

    #[test]
    fn test_table() {
        let table_type = TableType::new(RefType::FuncRef, Limits::Min(10));
        let mut table = Table::new(table_type);
        assert_eq!(table.size(), 10);
        assert_eq!(
            table.get(5),
            Some(super::super::WasmValue::NullRef(RefType::FuncRef))
        );
        table.set(5, super::super::WasmValue::FuncRef(42)).unwrap();
        assert_eq!(table.get(5), Some(super::super::WasmValue::FuncRef(42)));
    }

    #[test]
    fn test_global() {
        let global_type = GlobalType::new(ValType::Num(NumType::I32), true);
        let global = Global::new(global_type, super::super::WasmValue::I32(42)).unwrap();
        assert_eq!(global.get(), super::super::WasmValue::I32(42));
    }

    #[test]
    fn test_global_immutable() {
        let global_type = GlobalType::new(ValType::Num(NumType::I32), false);
        let mut global = Global::new(global_type, super::super::WasmValue::I32(42)).unwrap();
        assert!(global.set(super::super::WasmValue::I32(100)).is_err());
    }
}
