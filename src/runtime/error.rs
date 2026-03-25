#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrapCode {
    Unreachable,
    MemoryOutOfBounds,
    MemoryLimitExceeded,
    TableOutOfBounds,
    IndirectCallTypeMismatch,
    StackOverflow,
    ExecutionBudgetExceeded,
    IntegerOverflow,
    IntegerDivisionByZero,
    InvalidConversionToInt,
    CallIndirectNull,
    NullReference,
    HostTrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspensionKind {
    Safepoint,
    HostcallPending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmError {
    Validation(String),
    Load(String),
    Instantiate(String),
    Runtime(String),
    Suspended(SuspensionKind),
    Trap(TrapCode),
    Other(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::Validation(msg) => write!(f, "Validation error: {}", msg),
            WasmError::Load(msg) => write!(f, "Load error: {}", msg),
            WasmError::Instantiate(msg) => write!(f, "Instantiate error: {}", msg),
            WasmError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            WasmError::Suspended(SuspensionKind::Safepoint) => {
                write!(f, "Execution suspended at safepoint")
            }
            WasmError::Suspended(SuspensionKind::HostcallPending) => {
                write!(f, "Execution suspended for pending hostcall")
            }
            WasmError::Trap(code) => write!(f, "Trap: {:?}", code),
            WasmError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for WasmError {}

pub type Result<T> = std::result::Result<T, WasmError>;

impl From<std::io::Error> for WasmError {
    fn from(e: std::io::Error) -> Self {
        WasmError::Load(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = WasmError::Validation("type mismatch".to_string());
        assert_eq!(format!("{}", err), "Validation error: type mismatch");
    }

    #[test]
    fn test_suspension_display() {
        let err = WasmError::Suspended(SuspensionKind::Safepoint);
        assert_eq!(format!("{}", err), "Execution suspended at safepoint");
    }

    #[test]
    fn test_trap_code() {
        assert_eq!(
            format!("{:?}", TrapCode::MemoryOutOfBounds),
            "MemoryOutOfBounds"
        );
    }

    #[test]
    fn test_result_alias() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }
}
