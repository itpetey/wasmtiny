pub mod exec;
pub mod fast;
pub mod instructions;
pub mod stack;

pub use exec::Interpreter;
pub use fast::{FastInterpreter, IrBlock, IrOpcode};
pub use instructions::Instruction;
pub use stack::{ControlFrame, ControlStack, OperandStack};
