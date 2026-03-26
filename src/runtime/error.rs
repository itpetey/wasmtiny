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

/// Suspension kinds for async execution.
///
/// Indicates why execution was suspended (e.g., for safepoints or host calls).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Suspension kind.
pub enum SuspensionKind {
    /// Execution paused at a safepoint.
    Safepoint,
    /// Execution paused for pending host call.
    HostcallPending,
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
    /// Execution suspended (async support).
    Suspended(SuspensionKind),
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

/// Result type alias for WebAssembly operations.
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
