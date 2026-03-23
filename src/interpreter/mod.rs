pub mod fast;
pub mod instructions;
pub mod interpreter;
pub mod stack;

pub use fast::{FastInterpreter, IrBlock, IrOpcode};
pub use instructions::Instruction;
pub use interpreter::Interpreter;
pub use stack::{ControlFrame, ControlStack, OperandStack};
