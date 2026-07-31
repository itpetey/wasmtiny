/// Result type alias for WebAssembly operations.
pub type Result<T> = std::result::Result<T, WasmError>;

/// WebAssembly trap codes.
///
/// These codes indicate the specific reason for a WebAssembly trap, which
/// typically terminates execution.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Trap code.
pub enum TrapCode {
    /// Execution reached an unreachable instruction.
    Unreachable,
    /// Memory access outside bounds.
    MemoryOutOfBounds,
    /// Memory growth exceeded maximum.
    MemoryLimitExceeded,
    /// Table access outside bounds.
    TableOutOfBounds,
    /// Indirect call type mismatch.
    IndirectCallTypeMismatch,
    /// Stack overflow.
    StackOverflow,
    /// Execution budget exceeded (metering).
    ExecutionBudgetExceeded,
    /// Integer overflow in arithmetic operation.
    IntegerOverflow,
    /// Integer division by zero.
    IntegerDivisionByZero,
    /// Invalid conversion to integer (e.g., NaN).
    InvalidConversionToInt,
    /// Call indirect on null table entry.
    CallIndirectNull,
    /// Null reference used where non-null required.
    NullReference,
    /// Trap triggered by host.
    HostTrap,
}

/// WebAssembly errors.
///
/// Represents errors that can occur during validation, loading, instantiation,
/// or execution of WebAssembly modules.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Wasm error.
pub enum WasmError {
    /// Module validation failed.
    Validation(String),
    /// Module loading failed.
    Load(String),
    /// Module instantiation failed.
    Instantiate(String),
    /// Runtime error during execution.
    Runtime(String),
    /// Execution trapped.
    Trap(TrapCode),
    /// Other error.
    Other(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::Validation(msg) => write!(f, "Validation error: {}", msg),
            WasmError::Load(msg) => write!(f, "Load error: {}", msg),
            WasmError::Instantiate(msg) => write!(f, "Instantiate error: {}", msg),
            WasmError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            WasmError::Trap(code) => write!(f, "Trap: {:?}", code),
            WasmError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for WasmError {}

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
