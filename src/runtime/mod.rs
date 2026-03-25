mod error;
mod export;
mod import;
mod instance;
mod metering;
mod module;
mod suspend;
mod types;
mod values;

pub use crate::memory::Memory;
pub use error::{Result, SuspensionKind, TrapCode, WasmError};
pub use export::{ExportKind, ExportType};
pub use import::{Import, ImportKind, ImportType};
pub use instance::{
    Extern, HostCallOutcome, HostFunc, Instance, SharedGlobal, SharedMemory, SharedTable, Store,
};
pub(crate) use metering::InstanceMeter;
pub use metering::{InstanceLimits, InstanceStats};
pub use module::{DataKind, DataSegment, ElemKind, ElemSegment, Func, Local, Module};
#[cfg(feature = "llvm-jit")]
pub(crate) use suspend::JitState;
pub(crate) use suspend::{InterpreterState, SuspensionState};
pub use suspend::{RuntimeSuspender, SuspendedHandle, SuspensionError, is_suspension_error};
pub use types::{
    FunctionType, GlobalType, Limits, MemoryType, NumType, RefType, TableType, ValType,
};
pub use types::{Global, Table};
pub use values::WasmValue;
