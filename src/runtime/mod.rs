mod error;
mod export;
mod import;
mod instance;
mod module;
mod types;
mod values;

pub use crate::memory::Memory;
pub use error::{Result, TrapCode, WasmError};
pub use export::{ExportKind, ExportType};
pub use import::{Import, ImportKind, ImportType};
pub use instance::{Extern, HostFunc, Instance, Store};
pub use module::{DataKind, DataSegment, ElemKind, ElemSegment, Func, Local, Module};
pub use types::{
    FunctionType, GlobalType, Limits, MemoryType, NumType, RefType, TableType, ValType,
};
pub use types::{Global, Table};
pub use values::WasmValue;
