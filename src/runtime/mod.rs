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

pub(crate) use shared_memory::SharedWaiter;

pub use crate::memory::Memory;
pub use error::{Result, TrapCode, WasmError};
pub use export::{ExportKind, ExportType};
pub use import::{Import, ImportKind};
pub use instance::{
    Extern, GuestFuncBinding, HostCaller, HostFunc, Instance, SharedGlobal, SharedMemory,
    SharedTable, Store,
};
pub use module::{DataKind, DataSegment, ElemKind, ElemSegment, Func, Local, Module};
pub use shared_memory::{SharedMemoryRegistry, SharedRegion, SharedRegionId};
pub use types::{
    FunctionType, GlobalType, Limits, MemoryType, NumType, RefType, TableType, ValType,
};
pub use types::{Global, Table};
pub use values::WasmValue;

mod error;
mod export;
mod import;
mod instance;
mod module;
mod shared_memory;
mod types;
mod values;
