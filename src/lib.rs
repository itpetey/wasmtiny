//! A tiny WebAssembly runtime written in Rust.
//!
//! This library provides a Wasmtime-inspired API for loading, instantiating, and
//! executing WebAssembly modules. It supports both an interpreter mode and
//! (optionally) an LLVM-based JIT compiler.
//!
//! # Execution Modes
//!
//! - **Interpreter**: The default execution mode. Parses and executes WebAssembly
//!   bytecode directly.
//! - **LLVM JIT**: Available when the `llvm-jit` feature is enabled. Provides
//!   faster execution by compiling WebAssembly to native machine code.
//!
//! # Basic Usage
//!
//! ```ignore
//! use wasmtiny::{WasmApplication, WasmValue};
//!
//! // Create a new application
//! let mut app = WasmApplication::new();
//!
//! // Load a WebAssembly module
//! let module_idx = app.load_module_from_file("module.wasm")?;
//!
//! // Instantiate the module (resolves imports)
//! app.instantiate(module_idx)?;
//!
//! // Call a function
//! let result = app.call_function(module_idx, "add", &[WasmValue::I32(1), WasmValue::I32(2)])?;
//! assert_eq!(result, vec![WasmValue::I32(3)]);
//! ```
//!
//! # Feature Flags
//!
//! - `llvm-jit`: Enable LLVM-based JIT compilation for improved performance.
//!   Requires LLVM libraries to be installed.

pub mod aot_runtime;
/// Application APIs.
pub mod application;
/// Interpreter APIs.
pub mod interpreter;
/// Jit APIs.
pub mod jit;
/// Loader-related APIs.
pub mod loader;
/// Memory APIs.
pub mod memory;
/// Runtime-related APIs.
pub mod runtime;

pub use application::WasmApplication;
pub use interpreter::SafepointConfig;
pub use runtime::ExportType;
pub use runtime::FunctionType;
pub use runtime::Global;
pub use runtime::GlobalType;
pub use runtime::HostCallOutcome;
pub use runtime::ImportType;
pub use runtime::Instance;
pub use runtime::InstanceLimits;
pub use runtime::InstanceStats;
pub use runtime::Memory;
pub use runtime::MemoryType;
pub use runtime::Module;
pub use runtime::NumType;
pub use runtime::RefType;
pub use runtime::RuntimeSuspender;
pub use runtime::SharedMemoryMapping;
pub use runtime::SharedMemoryMappingId;
pub use runtime::SharedRegionId;
pub use runtime::SuspendedHandle;
pub use runtime::SuspensionError;
pub use runtime::SuspensionKind;
pub use runtime::Table;
pub use runtime::TableType;
pub use runtime::TrapCode;
pub use runtime::ValType;
pub use runtime::WasmError;
pub use runtime::WasmValue;
pub use runtime::is_suspension_error;

#[cfg(feature = "llvm-jit")]
pub use jit::LlvmJit;
