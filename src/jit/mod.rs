//! Just-In-Time (JIT) compilation for WebAssembly.
//!
//! This module provides JIT compilation capabilities to accelerate WebAssembly
//! execution. It includes both a custom JIT compiler and optional LLVM integration.
//!
//! # Components
//!
//! - [`JitCompiler`] - Custom JIT compiler for WebAssembly to native code
//! - [`Emitter`] - x86-64 instruction emitter
//! - [`LinearScanAllocator`] - Register allocator for JIT code
//! - [`JitCodeCache`] - Cache for generated JIT code
//! - [`LlvmJit`] - LLVM-based JIT compiler (requires `llvm-jit` feature)

pub use compiler::JitCompiler;
pub use emitter::{Address, Condition, Emitter, Reg, XmmReg};
#[cfg(feature = "llvm-jit")]
pub use llvm_backend::LlvmJit;
#[cfg(feature = "llvm-jit")]
pub use llvm_runtime::{set_execution_context, set_memory_context};
pub use regalloc::{LinearScanAllocator, LiveInterval, ValueLoc};
pub use runtime::JitCodeCache;

mod compiler;
mod emitter;
#[cfg(feature = "llvm-jit")]
mod llvm_backend;
#[cfg(feature = "llvm-jit")]
mod llvm_runtime;
mod regalloc;
mod runtime;
#[cfg(feature = "llvm-jit")]
mod wasm_to_llvm;
