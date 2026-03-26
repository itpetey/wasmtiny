//! WebAssembly interpreter implementation.
//!
//! This module provides the interpreter execution engine for WebAssembly bytecode.
//! The interpreter executes WebAssembly instructions directly without compilation.
//!
//! # Components
//!
//! - [`Interpreter`] - Main interpreter implementation with execution control
//! - [`Instruction`] - WebAssembly instruction representation
//! - [`OperandStack`] - Stack for WebAssembly values
//! - [`ControlStack`] - Stack for control flow frames (blocks, loops, functions)
//! - [`SafepointConfig`] - Configuration for execution suspension points

/// Interpreter execution support.
pub mod exec;
#[allow(dead_code)]
mod fast;
/// Decoded WebAssembly instruction types.
pub mod instructions;
/// Interpreter stack types.
pub mod stack;

pub use exec::{Interpreter, SafepointConfig};
pub use instructions::Instruction;
pub use stack::{ControlFrame, ControlStack, FrameKind, OperandStack};
