pub mod exec;
#[allow(dead_code)]
mod fast;
pub mod instructions;
pub mod stack;

pub use exec::{Interpreter, SafepointConfig};
pub use instructions::Instruction;
pub use stack::{ControlFrame, ControlStack, FrameKind, OperandStack};
