mod compiler;
mod emitter;
mod regalloc;
mod runtime;

#[cfg(feature = "llvm-jit")]
mod llvm_backend;
#[cfg(feature = "llvm-jit")]
mod llvm_runtime;
#[cfg(feature = "llvm-jit")]
mod wasm_to_llvm;

pub use compiler::JitCompiler;
pub use emitter::{Address, Condition, Emitter, Reg, XmmReg};
pub use regalloc::{LinearScanAllocator, LiveInterval, ValueLoc};
pub use runtime::JitCodeCache;

#[cfg(feature = "llvm-jit")]
pub use llvm_backend::LlvmJit;
#[cfg(feature = "llvm-jit")]
pub use llvm_runtime::{set_execution_context, set_memory_context};
