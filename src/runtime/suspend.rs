//! Runtime suspension and safepoint support.
//!
//! This module provides experimental APIs for suspending and resuming WebAssembly
//! execution at safepoints. This enables cooperative preemption and async hostcalls.
//!
//! # Stability
//!
//! These APIs are experimental and subject to change. The following are not yet
//! fully implemented:
//! - Mid-function JIT safepoints beyond entry-wrapper suspension
//! - Pending hostcall continuation for JIT-imported functions
//! - Cross-thread suspension
//!
//! # Usage
//!
//! For interpreter execution with safepoints:
//! ```ignore
//! let mut interpreter = Interpreter::new()
//!     .with_suspender(RuntimeSuspender::new())
//!     .with_safepoints(SafepointConfig::new(true));
//! ```
//!
//! When execution suspends, use the handle to resume:
//! ```ignore
//! if let Err(e) = result {
//!     if is_suspension_error(&e) {
//!         let handle = interpreter.take_suspended_handle().unwrap();
//!         // ... possibly resume later
//!         interpreter.try_resume(&handle)?;
//!         interpreter.continue_execution(module)?
//!     }
//! }
//! ```
//!
//! For async hostcalls, return `HostCallOutcome::Pending` from an imported host function:
//! ```ignore
//! let result = interpreter.execute_function(module, func_idx, args);
//! // hostcall pending -> take the stored handle and pending work
//! let handle = interpreter.take_suspended_handle().unwrap();
//! let pending = handle.pending_work().unwrap();
//! // ... do async work and obtain final hostcall results ...
//! interpreter.resume_hostcall(&handle, &hostcall_results)?;
//! let result = interpreter.continue_execution(module)?;
//! ```

use crate::interpreter::{ControlStack, OperandStack};
use crate::runtime::{ValType, WasmError, WasmValue};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

pub fn is_suspension_error(error: &WasmError) -> bool {
    matches!(error, WasmError::Suspended(_))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuspensionError {
    InstanceNotSuspended(String),
    InvalidResumeState(String),
    UnsupportedSuspensionState(String),
    HostcallPending,
}

impl std::fmt::Display for SuspensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InstanceNotSuspended(msg) => write!(f, "Instance not suspended: {}", msg),
            Self::InvalidResumeState(msg) => write!(f, "Invalid resume state: {}", msg),
            Self::UnsupportedSuspensionState(msg) => {
                write!(f, "Unsupported suspension state: {}", msg)
            }
            Self::HostcallPending => write!(f, "Hostcall is pending completion"),
        }
    }
}

impl std::error::Error for SuspensionError {}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum ExecutionEngine {
    Interpreter,
    Jit { func_idx: u32 },
}

#[derive(Debug, Clone)]
pub(crate) struct InterpreterState {
    pub(crate) pc: usize,
    pub(crate) locals: Vec<WasmValue>,
    pub(crate) operand_stack: OperandStack,
    pub(crate) control_stack: ControlStack,
    pub(crate) interpreter_id: u64,
    pub(crate) suspension_epoch: u64,
}

impl InterpreterState {
    pub fn capture(
        pc: usize,
        locals: Vec<WasmValue>,
        operand_stack: OperandStack,
        control_stack: ControlStack,
        interpreter_id: u64,
        suspension_epoch: u64,
    ) -> Self {
        Self {
            pc,
            locals,
            operand_stack,
            control_stack,
            interpreter_id,
            suspension_epoch,
        }
    }

    pub fn restore(&self) -> (usize, Vec<WasmValue>, OperandStack, ControlStack) {
        (
            self.pc,
            self.locals.clone(),
            self.operand_stack.clone(),
            self.control_stack.clone(),
        )
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) enum JitState {
    Pending {
        func_idx: u32,
        args: Vec<WasmValue>,
        jit_id: u64,
        execution_epoch: u64,
        context_id: u64,
        resume_pc: u32,
    },
    Suspended {
        func_idx: u32,
        jit_id: u64,
        execution_epoch: u64,
        context_id: u64,
        saved_registers: Vec<u64>,
        stack_pointer: u64,
        frame_pointer: u64,
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) enum SuspensionState {
    Interpreter(InterpreterState),
    Jit(JitState),
    HostcallPending {
        pending_work: Vec<u8>,
        result_types: Vec<ValType>,
        resume_state: Box<SuspensionState>,
    },
    #[default]
    None,
}

struct SuspendedInstance {
    engine: ExecutionEngine,
    state: Arc<RwLock<SuspensionState>>,
    suspended: Arc<AtomicBool>,
    instance_id: u64,
}

impl std::fmt::Debug for SuspendedInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuspendedInstance")
            .field("engine", &self.engine)
            .field("state", &self.state)
            .field("suspended", &self.suspended.load(Ordering::SeqCst))
            .field("instance_id", &self.instance_id)
            .finish()
    }
}

impl Clone for SuspendedInstance {
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
            state: self.state.clone(),
            suspended: self.suspended.clone(),
            instance_id: self.instance_id,
        }
    }
}

impl SuspendedInstance {
    fn new_interpreter(state: InterpreterState, instance_id: u64) -> Self {
        Self {
            engine: ExecutionEngine::Interpreter,
            state: Arc::new(RwLock::new(SuspensionState::Interpreter(state))),
            suspended: Arc::new(AtomicBool::new(true)),
            instance_id,
        }
    }

    fn new_jit(
        func_idx: u32,
        args: Vec<WasmValue>,
        jit_id: u64,
        execution_epoch: u64,
        context_id: u64,
        instance_id: u64,
    ) -> Self {
        Self {
            engine: ExecutionEngine::Jit { func_idx },
            state: Arc::new(RwLock::new(SuspensionState::Jit(JitState::Pending {
                func_idx,
                args,
                jit_id,
                execution_epoch,
                context_id,
                resume_pc: 0,
            }))),
            suspended: Arc::new(AtomicBool::new(true)),
            instance_id,
        }
    }

    fn is_suspended(&self) -> bool {
        self.suspended.load(Ordering::SeqCst)
    }

    fn engine(&self) -> &ExecutionEngine {
        &self.engine
    }

    fn state(&self) -> Arc<RwLock<SuspensionState>> {
        self.state.clone()
    }

    fn instance_id(&self) -> u64 {
        self.instance_id
    }

    pub(crate) fn resume(&self) -> Result<SuspensionState, SuspensionError> {
        let state = {
            let guard = self.state.read();
            if matches!(*guard, SuspensionState::HostcallPending { .. }) {
                return Err(SuspensionError::HostcallPending);
            }
            match &*guard {
                SuspensionState::None => {
                    return Err(SuspensionError::InvalidResumeState(
                        "no suspension state to resume".to_string(),
                    ));
                }
                _ => guard.clone(),
            }
        };

        let result =
            self.suspended
                .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst);

        match result {
            Ok(_) => Ok(state),
            Err(_) => Err(SuspensionError::InstanceNotSuspended(
                "instance is not suspended".to_string(),
            )),
        }
    }

    /// Atomically complete a pending hostcall and resume execution.
    ///
    /// This is the only valid way to resume from a HostcallPending state.
    /// It prevents races between completing the hostcall and resuming.
    ///
    /// This operation keeps the pending state intact if the instance has already
    /// been resumed.
    pub(crate) fn resume_after_hostcall(
        &self,
        results: &[WasmValue],
    ) -> Result<SuspensionState, SuspensionError> {
        let mut guard = self.state.write();

        if !self.suspended.load(Ordering::SeqCst) {
            return Err(SuspensionError::InstanceNotSuspended(
                "instance is not suspended".to_string(),
            ));
        }

        let state = match &mut *guard {
            SuspensionState::HostcallPending {
                result_types,
                resume_state,
                ..
            } => {
                validate_hostcall_results(results, result_types)?;
                let restored = apply_hostcall_results(*resume_state.clone(), results)?;
                *guard = restored.clone();
                self.suspended.store(false, Ordering::SeqCst);
                restored
            }
            _ => {
                return Err(SuspensionError::InvalidResumeState(
                    "not in hostcall pending state".to_string(),
                ));
            }
        };

        Ok(state)
    }

    fn with_pending_hostcall(pending_work: Vec<u8>, resume_state: SuspensionState) -> Self {
        let instance = Self {
            engine: ExecutionEngine::Interpreter,
            state: Arc::new(RwLock::new(SuspensionState::None)),
            suspended: Arc::new(AtomicBool::new(true)),
            instance_id: 0,
        };
        *instance.state.write() = SuspensionState::HostcallPending {
            pending_work,
            result_types: Vec::new(),
            resume_state: Box::new(resume_state),
        };
        instance
    }

    fn set_instance_id(&mut self, id: u64) {
        self.instance_id = id;
    }
}

#[derive(Debug)]
pub struct SuspendedHandle(Arc<SuspendedInstance>);

impl SuspendedHandle {
    fn new(instance: SuspendedInstance) -> Self {
        Self(Arc::new(instance))
    }

    pub fn is_suspended(&self) -> bool {
        self.0.is_suspended()
    }

    pub fn has_pending_hostcall(&self) -> bool {
        let state = self.0.state();
        let guard = state.read();
        matches!(*guard, SuspensionState::HostcallPending { .. })
    }

    pub(crate) fn interpreter_id(&self) -> Option<u64> {
        let state = self.0.state();
        let guard = state.read();
        match &*guard {
            SuspensionState::Interpreter(state) => Some(state.interpreter_id),
            SuspensionState::HostcallPending { resume_state, .. } => {
                if let SuspensionState::Interpreter(state) = &**resume_state {
                    Some(state.interpreter_id)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn suspension_epoch(&self) -> Option<u64> {
        let state = self.0.state();
        let guard = state.read();
        match &*guard {
            SuspensionState::Interpreter(state) => Some(state.suspension_epoch),
            SuspensionState::HostcallPending { resume_state, .. } => {
                if let SuspensionState::Interpreter(state) = &**resume_state {
                    Some(state.suspension_epoch)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn jit_id(&self) -> Option<u64> {
        let state = self.0.state();
        let guard = state.read();
        match &*guard {
            SuspensionState::Jit(JitState::Pending { jit_id, .. }) => Some(*jit_id),
            SuspensionState::Jit(JitState::Suspended { jit_id, .. }) => Some(*jit_id),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn jit_execution_epoch(&self) -> Option<u64> {
        let state = self.0.state();
        let guard = state.read();
        match &*guard {
            SuspensionState::Jit(JitState::Pending {
                execution_epoch, ..
            }) => Some(*execution_epoch),
            SuspensionState::Jit(JitState::Suspended {
                execution_epoch, ..
            }) => Some(*execution_epoch),
            _ => None,
        }
    }

    pub fn pending_work(&self) -> Option<Vec<u8>> {
        let state = self.0.state();
        let guard = state.read();
        if let SuspensionState::HostcallPending { pending_work, .. } = &*guard {
            Some(pending_work.clone())
        } else {
            None
        }
    }

    pub(crate) fn resume_after_hostcall(
        &self,
        results: &[WasmValue],
    ) -> Result<SuspensionState, SuspensionError> {
        self.0.resume_after_hostcall(results)
    }

    pub(crate) fn resume(&self) -> Result<SuspensionState, SuspensionError> {
        self.0.resume()
    }

    #[allow(dead_code)]
    pub(crate) fn engine(&self) -> &ExecutionEngine {
        self.0.engine()
    }

    pub fn instance_id(&self) -> u64 {
        self.0.instance_id()
    }
}

impl From<SuspendedInstance> for SuspendedHandle {
    fn from(instance: SuspendedInstance) -> Self {
        SuspendedHandle::new(instance)
    }
}

pub struct RuntimeSuspender;

impl RuntimeSuspender {
    pub fn new() -> Self {
        Self
    }

    pub(crate) fn suspend_interpreter(
        &self,
        pc: usize,
        locals: Vec<WasmValue>,
        operand_stack: OperandStack,
        control_stack: ControlStack,
        interpreter_id: u64,
        suspension_epoch: u64,
    ) -> SuspendedHandle {
        let state = InterpreterState::capture(
            pc,
            locals,
            operand_stack,
            control_stack,
            interpreter_id,
            suspension_epoch,
        );
        let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
        let instance = SuspendedInstance::new_interpreter(state, instance_id);
        SuspendedHandle::new(instance)
    }

    #[allow(dead_code)]
    pub(crate) fn suspend_jit(
        &self,
        func_idx: u32,
        args: Vec<WasmValue>,
        jit_id: u64,
        execution_epoch: u64,
        context_id: u64,
    ) -> SuspendedHandle {
        let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
        let instance = SuspendedInstance::new_jit(
            func_idx,
            args,
            jit_id,
            execution_epoch,
            context_id,
            instance_id,
        );
        SuspendedHandle::new(instance)
    }

    pub(crate) fn suspend_with_pending_hostcall(
        &self,
        pending_work: Vec<u8>,
        result_types: Vec<ValType>,
        state: InterpreterState,
    ) -> SuspendedHandle {
        let resume_state = SuspensionState::Interpreter(state);
        let instance_id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::SeqCst);
        let mut instance = SuspendedInstance::with_pending_hostcall(pending_work, resume_state);
        if let SuspensionState::HostcallPending {
            result_types: expected,
            ..
        } = &mut *instance.state.write()
        {
            *expected = result_types;
        }
        instance.set_instance_id(instance_id);
        SuspendedHandle::new(instance)
    }

    pub fn is_suspended(handle: &SuspendedHandle) -> bool {
        handle.is_suspended()
    }
}

impl Default for RuntimeSuspender {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_hostcall_results(
    results: &[WasmValue],
    expected: &[ValType],
) -> Result<(), SuspensionError> {
    if results.len() != expected.len() {
        return Err(SuspensionError::InvalidResumeState(format!(
            "hostcall result count mismatch: expected {}, got {}",
            expected.len(),
            results.len()
        )));
    }

    for (index, (value, value_type)) in results.iter().zip(expected.iter()).enumerate() {
        if value.val_type() != *value_type {
            return Err(SuspensionError::InvalidResumeState(format!(
                "hostcall result {} type mismatch: expected {:?}, got {:?}",
                index,
                value_type,
                value.val_type()
            )));
        }
    }

    Ok(())
}

fn apply_hostcall_results(
    mut state: SuspensionState,
    results: &[WasmValue],
) -> Result<SuspensionState, SuspensionError> {
    match &mut state {
        SuspensionState::Interpreter(interpreter_state) => {
            for value in results {
                interpreter_state
                    .operand_stack
                    .push(*value)
                    .map_err(|e| SuspensionError::InvalidResumeState(e.to_string()))?;
            }
            Ok(state)
        }
        _ => Err(SuspensionError::UnsupportedSuspensionState(
            "hostcall completion is only supported for interpreter state".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::NumType;

    #[test]
    fn test_suspended_instance_creation() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        assert!(instance.is_suspended());
        assert!(matches!(instance.engine(), ExecutionEngine::Interpreter));
    }

    #[test]
    fn test_suspended_handle_resume() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 10,
                locals: vec![WasmValue::I32(42)],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);
        assert!(handle.is_suspended());

        let state = handle.resume().unwrap();
        assert!(matches!(state, SuspensionState::Interpreter(_)));
    }

    #[test]
    fn test_resume_fails_when_not_suspended() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);

        handle.resume().unwrap();
        assert!(!handle.is_suspended());

        let result = handle.resume();
        assert!(matches!(
            result,
            Err(SuspensionError::InstanceNotSuspended(_))
        ));
    }

    #[test]
    fn test_unsupported_state_error() {
        let error =
            SuspensionError::UnsupportedSuspensionState("preemption not supported".to_string());
        assert!(format!("{}", error).contains("Unsupported suspension state"));
    }

    #[test]
    fn test_locals_preserved_after_suspend() {
        let locals = vec![WasmValue::I32(1), WasmValue::I64(2), WasmValue::F32(3.0)];
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 100,
                locals: locals.clone(),
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);

        let state = handle.resume().unwrap();
        if let SuspensionState::Interpreter(state) = state {
            assert_eq!(state.locals, locals);
        } else {
            panic!("expected interpreter state");
        }
    }

    #[test]
    fn test_stack_preserved_after_suspend() {
        let mut operand_stack = OperandStack::new(1024);
        operand_stack.push(WasmValue::I32(10)).unwrap();
        operand_stack.push(WasmValue::I32(20)).unwrap();
        operand_stack.push(WasmValue::I64(100)).unwrap();

        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 50,
                locals: vec![],
                operand_stack,
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);

        let state = handle.resume().unwrap();
        if let SuspensionState::Interpreter(state) = state {
            assert_eq!(state.operand_stack.len(), 3);
        } else {
            panic!("expected interpreter state");
        }
    }

    #[test]
    fn test_pc_preserved_after_suspend() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 1234,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 1,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);

        let state = handle.resume().unwrap();
        if let SuspensionState::Interpreter(state) = state {
            assert_eq!(state.pc, 1234);
        } else {
            panic!("expected interpreter state");
        }
    }

    #[test]
    fn test_runtime_suspender_creates_handles() {
        let suspender = RuntimeSuspender::new();

        let handle = suspender.suspend_interpreter(
            100,
            vec![WasmValue::I32(42)],
            OperandStack::new(1024),
            ControlStack::new(),
            0,
            0,
        );

        assert!(handle.is_suspended());
        assert!(matches!(handle.engine(), ExecutionEngine::Interpreter));
    }

    #[test]
    fn test_jit_suspension_state() {
        let instance = SuspendedInstance::new_jit(5, vec![WasmValue::I32(7)], 11, 23, 99, 1);

        assert!(instance.is_suspended());
        if let ExecutionEngine::Jit { func_idx } = instance.engine() {
            assert_eq!(*func_idx, 5);
        } else {
            panic!("expected JIT engine");
        }

        match &*instance.state.read() {
            SuspensionState::Jit(JitState::Pending {
                args,
                jit_id,
                execution_epoch,
                context_id,
                ..
            }) => {
                assert_eq!(args, &vec![WasmValue::I32(7)]);
                assert_eq!(*jit_id, 11);
                assert_eq!(*execution_epoch, 23);
                assert_eq!(*context_id, 99);
            }
            _ => panic!("expected pending JIT state"),
        }
    }

    #[test]
    fn test_hostcall_pending_state() {
        let suspender = RuntimeSuspender::new();
        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3, 4],
            vec![ValType::Num(NumType::I32)],
            InterpreterState {
                pc: 50,
                locals: vec![WasmValue::I32(1)],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
        );

        let result = handle.resume_after_hostcall(&[WasmValue::I32(7)]);
        assert!(matches!(result, Ok(SuspensionState::Interpreter(_))));
    }

    #[test]
    fn test_double_resume_fails() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 1,
                suspension_epoch: 0,
            },
            1,
        );
        let state = instance.state();
        *state.write() = SuspensionState::None;

        let handle = SuspendedHandle::new(instance);
        let result = handle.resume();

        assert!(matches!(
            result,
            Err(SuspensionError::InvalidResumeState(_))
        ));
    }

    #[test]
    fn test_unsupported_suspension_state_error_message() {
        let error = SuspensionError::UnsupportedSuspensionState("nested suspension".to_string());
        assert_eq!(
            format!("{}", error),
            "Unsupported suspension state: nested suspension"
        );
    }

    #[test]
    fn test_instance_not_suspended_error_message() {
        let error = SuspensionError::InstanceNotSuspended("already resumed".to_string());
        assert_eq!(
            format!("{}", error),
            "Instance not suspended: already resumed"
        );
    }

    #[test]
    fn test_invalid_resume_state_error_message() {
        let error = SuspensionError::InvalidResumeState("corrupted state".to_string());
        assert_eq!(
            format!("{}", error),
            "Invalid resume state: corrupted state"
        );
    }

    #[test]
    fn test_hostcall_pending_error_message() {
        let error = SuspensionError::HostcallPending;
        assert_eq!(format!("{}", error), "Hostcall is pending completion");
    }

    #[test]
    fn test_instance_ids_are_unique() {
        let suspender = RuntimeSuspender::new();

        let handle1 = suspender.suspend_interpreter(
            0,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            0,
            0,
        );
        let handle2 = suspender.suspend_interpreter(
            0,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        assert_ne!(handle1.instance_id(), handle2.instance_id());
    }

    #[test]
    fn test_hostcall_resume_state_contains_pending_work() {
        let suspender = RuntimeSuspender::new();
        let handle = suspender.suspend_with_pending_hostcall(
            vec![0xDE, 0xAD, 0xBE, 0xEF],
            vec![ValType::Num(NumType::I32)],
            InterpreterState {
                pc: 100,
                locals: vec![WasmValue::I32(42)],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
        );

        if let Ok(SuspensionState::Interpreter(state)) =
            handle.resume_after_hostcall(&[WasmValue::I32(9)])
        {
            assert_eq!(state.pc, 100);
            assert_eq!(state.locals.len(), 1);
            let mut operand_stack = state.operand_stack;
            assert_eq!(operand_stack.pop(), Some(WasmValue::I32(9)));
        } else {
            panic!("expected Interpreter state");
        }
    }

    #[test]
    fn test_has_pending_hostcall() {
        let pending_work = vec![1, 2, 3];
        let resume_state = SuspensionState::Interpreter(InterpreterState {
            pc: 0,
            locals: vec![],
            operand_stack: OperandStack::new(1024),
            control_stack: ControlStack::new(),
            interpreter_id: 0,
            suspension_epoch: 0,
        });

        let instance = SuspendedInstance::with_pending_hostcall(pending_work, resume_state);
        let handle = SuspendedHandle::new(instance);

        assert!(handle.has_pending_hostcall());
        assert_eq!(handle.pending_work(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_no_pending_hostcall_on_interpreter_state() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(instance);

        assert!(!handle.has_pending_hostcall());
        assert!(handle.pending_work().is_none());
    }

    #[test]
    fn test_is_suspended_after_external_resume() {
        let instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: OperandStack::new(1024),
                control_stack: ControlStack::new(),
                interpreter_id: 0,
                suspension_epoch: 0,
            },
            1,
        );
        let instance2 = instance.clone();
        let handle = SuspendedHandle::new(instance);
        let handle2 = SuspendedHandle::new(instance2);

        assert!(handle.is_suspended());

        // Resume from a different handle (simulating external resume)
        handle2.resume().unwrap();

        // Original handle should now report not suspended (they share the same underlying state)
        assert!(!handle.is_suspended());
    }
}
