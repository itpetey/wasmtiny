//! Core runtime types and utilities for WebAssembly execution.
//!
//! This module contains the fundamental types used throughout the runtime,
//! including value types, function types, memory/table types, error handling,
//! and instance management.
//!
//! # Key Types
//!
//! - [`WasmValue`] - Represents WebAssembly runtime values (i32, i64, f32, f64, refs)
//! - [`FunctionType`] - Function signature with parameters and results
//! - [`MemoryType`], [`TableType`], [`GlobalType`] - WebAssembly type definitions
//! - [`Module`] - A parsed WebAssembly module
//! - [`Instance`] - An instantiated module with runtime state
//! - [`WasmError`] - Error types for validation, loading, instantiation, and runtime
//! - [`Memory`], [`Table`], [`Global`] - Runtime objects

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
    Extern, GuestFuncBinding, HostCallOutcome, HostFunc, Instance, SharedGlobal, SharedMemory,
    SharedTable, Store,
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
