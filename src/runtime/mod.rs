mod error;
mod export;
mod import;
mod instance;
mod metering;
mod module;
mod shared_memory;
mod snapshot;
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
pub(crate) use shared_memory::{ResolvedSharedMemoryMapping, SharedMemoryRegistry};
pub use shared_memory::{SharedMemoryMapping, SharedMemoryMappingId, SharedRegionId};
pub use snapshot::{
    Result as SnapshotResult, SNAPSHOT_FORMAT_VERSION, SnapshotError, SnapshotPayload,
    capture_snapshot, restore_snapshot, validate_snapshot_compatibility,
};
#[cfg(feature = "llvm-jit")]
pub(crate) use suspend::JitState;
pub(crate) use suspend::{InterpreterState, SuspensionState};
pub use suspend::{RuntimeSuspender, SuspendedHandle, SuspensionError, is_suspension_error};
pub use types::{
    FunctionType, GlobalType, Limits, MemoryType, NumType, RefType, TableType, ValType,
};
pub use types::{Global, Table};
pub use values::WasmValue;
