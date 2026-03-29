use crate::interpreter::{ControlFrame, ControlStack, FrameKind, OperandStack};
use crate::runtime::{
    FunctionType, HostCallOutcome, Instance, Module, NumType, RefType, Result, RuntimeSuspender,
    SuspendedHandle, SuspensionKind, TrapCode, ValType, WasmError, WasmValue,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

const MAX_STACK_SIZE: usize = 16384;

static NEXT_INTERPRETER_ID: AtomicU64 = AtomicU64::new(1);

/// Configuration for interpreter safepoints.
///
/// Safepoints are points in the execution where the interpreter can be suspended
/// for async operations (e.g., host calls, cooperative multitasking).
#[derive(Clone)]
/// Safepoint config.
pub struct SafepointConfig {
    check_interval: u32,
    enabled: Arc<AtomicBool>,
}

impl SafepointConfig {
    /// Creates a new `SafepointConfig`.
    pub fn new(enabled: bool) -> Self {
        Self {
            check_interval: 100,
            enabled: Arc::new(AtomicBool::new(enabled)),
        }
    }

    /// Returns this value configured with interval.
    pub fn with_interval(mut self, interval: u32) -> Self {
        self.check_interval = interval;
        self
    }

    /// Returns whether enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Enables safepoint checks.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disables safepoint checks.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }
}

impl Default for SafepointConfig {
    fn default() -> Self {
        Self::new(false)
    }
}

struct ControlSplit {
    then_body: Vec<u8>,
    else_body: Option<Vec<u8>>,
    after_end: usize,
}

struct BlockSignature {
    param_count: usize,
    result_count: usize,
}

/// WebAssembly interpreter state and execution engine.
pub struct Interpreter {
    /// The operand stack for WebAssembly values.
    pub operand_stack: OperandStack,
    /// The control flow stack (for blocks, loops, functions).
    pub control_stack: ControlStack,
    /// The WebAssembly instance being executed.
    pub instance: Option<Arc<Mutex<Instance>>>,
    /// Local variables for the current function.
    pub locals: Vec<WasmValue>,
    safepoint_config: SafepointConfig,
    instruction_count: u32,
    suspender: Option<RuntimeSuspender>,
    suspended_handle: Option<SuspendedHandle>,
    active_suspension_id: Option<u64>,
    safepoint_armed: bool,
    resume_skip_pc: Option<usize>,
    interpreter_id: u64,
    suspension_epoch: u64,
    needs_resume: bool,
    execution_thread: Option<ThreadId>,
}

impl Interpreter {
    /// Creates a new `Interpreter`.
    pub fn new() -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: None,
            locals: Vec::new(),
            safepoint_config: SafepointConfig::default(),
            instruction_count: 0,
            suspender: None,
            suspended_handle: None,
            active_suspension_id: None,
            safepoint_armed: false,
            resume_skip_pc: None,
            interpreter_id: NEXT_INTERPRETER_ID.fetch_add(1, Ordering::SeqCst),
            suspension_epoch: 0,
            needs_resume: false,
            execution_thread: None,
        }
    }

    /// Returns this value configured with instance.
    pub fn with_instance(instance: Arc<Mutex<Instance>>) -> Self {
        Self {
            operand_stack: OperandStack::new(MAX_STACK_SIZE),
            control_stack: ControlStack::new(),
            instance: Some(instance),
            locals: Vec::new(),
            safepoint_config: SafepointConfig::default(),
            instruction_count: 0,
            suspender: None,
            suspended_handle: None,
            active_suspension_id: None,
            safepoint_armed: false,
            resume_skip_pc: None,
            interpreter_id: NEXT_INTERPRETER_ID.fetch_add(1, Ordering::SeqCst),
            suspension_epoch: 0,
            needs_resume: false,
            execution_thread: None,
        }
    }

    /// Returns this value configured with safepoints.
    pub fn with_safepoints(mut self, config: SafepointConfig) -> Self {
        self.safepoint_armed = config.is_enabled();
        self.safepoint_config = config;
        self
    }

    /// Returns this value configured with suspender.
    pub fn with_suspender(mut self, suspender: RuntimeSuspender) -> Self {
        self.suspender = Some(suspender);
        self
    }

    /// Executes the requested function.
    pub fn execute(&mut self, module: &Module, func_idx: u32) -> Result<Vec<WasmValue>> {
        self.execute_function(module, func_idx, &[])
    }

    /// Executes function.
    pub fn execute_function(
        &mut self,
        module: &Module,
        func_idx: u32,
        args: &[WasmValue],
    ) -> Result<Vec<WasmValue>> {
        if self.is_suspended() {
            return Err(WasmError::Runtime(
                "cannot start a new execution while suspended state is pending".to_string(),
            ));
        }

        if self.safepoint_config.is_enabled() && self.suspender.is_none() {
            return Err(WasmError::Runtime(
                "safepoints require a configured runtime suspender".to_string(),
            ));
        }

        self.control_stack.clear();
        self.operand_stack.clear();
        self.locals.clear();
        self.instruction_count = 0;
        self.suspended_handle = None;
        self.active_suspension_id = None;
        self.safepoint_armed = self.safepoint_config.is_enabled();
        self.resume_skip_pc = None;
        self.needs_resume = false;
        self.suspension_epoch += 1;
        self.execution_thread = Some(std::thread::current().id());

        let mut frame = self.build_frame(module, func_idx, args)?;
        frame.height = self.operand_stack.len();
        self.locals = frame.locals.clone();
        self.control_stack.push(frame);

        self.run(module)
    }

    /// Enables safepoints.
    pub fn enable_safepoints(&mut self) {
        self.safepoint_config.enable();
        self.safepoint_armed = true;
    }

    /// Disables safepoints.
    pub fn disable_safepoints(&mut self) {
        self.safepoint_config.disable();
        self.safepoint_armed = false;
    }

    /// Returns whether safepoint enabled.
    pub fn is_safepoint_enabled(&self) -> bool {
        self.safepoint_config.is_enabled()
    }

    /// Returns whether suspended.
    pub fn is_suspended(&self) -> bool {
        self.needs_resume || self.active_suspension_id.is_some()
    }

    /// Takes suspended handle.
    pub fn take_suspended_handle(&mut self) -> Option<SuspendedHandle> {
        self.suspended_handle.take()
    }

    /// Returns the current suspended execution handle, if any.
    pub fn suspended_handle(&self) -> Option<&SuspendedHandle> {
        self.suspended_handle.as_ref()
    }

    fn check_safepoint(&mut self) -> Option<SuspendedHandle> {
        if !self.safepoint_config.is_enabled() {
            return None;
        }

        if !self.safepoint_armed {
            return None;
        }

        if let Some(skip_pc) = self.resume_skip_pc {
            let current_pc = self
                .control_stack
                .last()
                .map(|frame| frame.position)
                .unwrap_or(0);
            self.resume_skip_pc = None;
            if current_pc == skip_pc {
                return None;
            }
        }

        self.instruction_count += 1;
        if self.instruction_count >= self.safepoint_config.check_interval {
            self.instruction_count = 0;
            if self.suspender.is_some() {
                let suspended = self.try_suspend();
                if suspended.is_some() {
                    self.safepoint_armed = false;
                }
                return suspended;
            }
        }
        None
    }

    fn try_suspend(&mut self) -> Option<SuspendedHandle> {
        let suspender = self.suspender.as_ref()?;

        let pc = self.control_stack.last().map(|f| f.position).unwrap_or(0);
        let locals = self.locals.clone();
        let epoch = self.suspension_epoch;

        Some(suspender.suspend_interpreter(
            pc,
            locals,
            self.operand_stack.clone(),
            self.control_stack.clone(),
            self.interpreter_id,
            epoch,
        ))
    }

    // Capture the current interpreter state at a hostcall boundary and mark the
    // interpreter as suspended until the hostcall is completed.
    fn suspend_hostcall_state_at(
        &mut self,
        pending_work: Vec<u8>,
        result_types: Vec<ValType>,
        resume_pc: usize,
    ) -> std::result::Result<(), crate::runtime::SuspensionError> {
        if self.needs_resume {
            return Err(crate::runtime::SuspensionError::UnsupportedSuspensionState(
                "interpreter is already suspended".to_string(),
            ));
        }

        let suspender = self.suspender.as_ref().ok_or_else(|| {
            crate::runtime::SuspensionError::UnsupportedSuspensionState(
                "runtime suspender is not configured".to_string(),
            )
        })?;

        let Some(frame) = self.control_stack.last() else {
            return Err(crate::runtime::SuspensionError::UnsupportedSuspensionState(
                "interpreter is not currently executing".to_string(),
            ));
        };

        if frame.position >= frame.code.len() {
            return Err(crate::runtime::SuspensionError::UnsupportedSuspensionState(
                "interpreter is not at a resumable hostcall boundary".to_string(),
            ));
        }

        let state = crate::runtime::InterpreterState::capture(
            resume_pc,
            self.locals.clone(),
            self.operand_stack.clone(),
            self.control_stack.clone(),
            self.interpreter_id,
            self.suspension_epoch,
        );

        let handle = suspender.suspend_with_pending_hostcall(pending_work, result_types, state);
        self.active_suspension_id = Some(handle.instance_id());
        self.suspended_handle = Some(handle);
        self.safepoint_armed = false;
        self.needs_resume = true;
        Ok(())
    }

    fn restore_interpreter_state(&mut self, interpreter_state: crate::runtime::InterpreterState) {
        let (pc, locals, operand_stack, control_stack) = interpreter_state.restore();
        self.control_stack = control_stack;
        self.operand_stack = operand_stack;
        self.locals = locals;
        if let Some(frame) = self.control_stack.last_mut() {
            frame.position = pc;
        }
        self.suspended_handle = None;
        self.active_suspension_id = None;
        self.safepoint_armed = self.safepoint_config.is_enabled();
        self.resume_skip_pc = Some(pc);
        self.instruction_count = 0;
        self.needs_resume = false;
    }

    fn validate_suspended_handle(
        &self,
        handle: &SuspendedHandle,
    ) -> std::result::Result<(), crate::runtime::WasmError> {
        if let Some(thread_id) = self.execution_thread
            && thread_id != std::thread::current().id()
        {
            return Err(crate::runtime::WasmError::Runtime(
                "cross-thread interpreter resume is unsupported".to_string(),
            ));
        }

        if let Some(suspended_interpreter_id) = handle.interpreter_id()
            && suspended_interpreter_id != self.interpreter_id
        {
            return Err(crate::runtime::WasmError::Runtime(
                "suspended handle is from a different interpreter".to_string(),
            ));
        }

        if let Some(suspended_epoch) = handle.suspension_epoch()
            && suspended_epoch != self.suspension_epoch
        {
            return Err(crate::runtime::WasmError::Runtime(
                "suspended handle is from a previous execution epoch".to_string(),
            ));
        }

        if let Some(active_suspension_id) = self.active_suspension_id
            && handle.instance_id() != active_suspension_id
        {
            return Err(crate::runtime::WasmError::Runtime(
                "suspended handle does not match the active suspension".to_string(),
            ));
        }

        Ok(())
    }

    /// Attempts to resume execution from a suspended handle.
    pub fn try_resume(
        &mut self,
        handle: &SuspendedHandle,
    ) -> std::result::Result<(), crate::runtime::WasmError> {
        self.validate_suspended_handle(handle)?;

        if handle.has_pending_hostcall() {
            return Err(crate::runtime::WasmError::Runtime(
                "hostcall resume requires completion results".to_string(),
            ));
        }

        let state = handle
            .resume()
            .map_err(|e| crate::runtime::WasmError::Runtime(format!("resume failed: {}", e)))?;

        match state {
            crate::runtime::SuspensionState::Interpreter(interpreter_state) => {
                self.restore_interpreter_state(interpreter_state);
                Ok(())
            }
            _ => Err(crate::runtime::WasmError::Runtime(
                "invalid resume state".to_string(),
            )),
        }
    }

    /// Complete a pending hostcall with final results and resume execution.
    ///
    /// This restores the suspended interpreter state, injects the hostcall
    /// results onto the operand stack, and resumes at the next instruction.
    pub fn resume_hostcall(
        &mut self,
        handle: &SuspendedHandle,
        results: &[WasmValue],
    ) -> std::result::Result<(), crate::runtime::WasmError> {
        if !handle.has_pending_hostcall() {
            return Err(crate::runtime::WasmError::Runtime(
                "handle does not contain a pending hostcall".to_string(),
            ));
        }

        self.validate_suspended_handle(handle)?;

        let state = handle
            .resume_after_hostcall(results)
            .map_err(|e| crate::runtime::WasmError::Runtime(format!("resume failed: {}", e)))?;

        match state {
            crate::runtime::SuspensionState::Interpreter(interpreter_state) => {
                self.restore_interpreter_state(interpreter_state);
                Ok(())
            }
            _ => Err(crate::runtime::WasmError::Runtime(
                "invalid resume state".to_string(),
            )),
        }
    }

    /// Continues execution after a suspension point.
    pub fn continue_execution(&mut self, module: &Module) -> Result<Vec<WasmValue>> {
        if let Some(thread_id) = self.execution_thread
            && thread_id != std::thread::current().id()
        {
            return Err(WasmError::Runtime(
                "cross-thread interpreter continue is unsupported".to_string(),
            ));
        }

        if self.needs_resume {
            return Err(WasmError::Runtime(
                "cannot continue: suspended handle must be resumed first".to_string(),
            ));
        }
        if self.control_stack.is_empty() {
            return Err(WasmError::Runtime(
                "no suspended execution is available to continue".to_string(),
            ));
        }
        self.run(module)
    }

    fn run(&mut self, module: &Module) -> Result<Vec<WasmValue>> {
        loop {
            if let Some(suspended) = self.check_safepoint() {
                self.active_suspension_id = Some(suspended.instance_id());
                self.suspended_handle = Some(suspended);
                self.needs_resume = true;
                return Err(WasmError::Suspended(SuspensionKind::Safepoint));
            }

            let should_finish = match self.control_stack.last() {
                Some(frame) => frame.position >= frame.code.len(),
                None => return Ok(Vec::new()),
            };

            if should_finish {
                if let Some(results) = self.finish_frame()? {
                    return Ok(results);
                }
                continue;
            }

            let opcode = self.read_u8_immediate()?;
            self.record_execution(1)?;
            match opcode {
                0x0B => {
                    if let Some(results) = self.finish_frame()? {
                        return Ok(results);
                    }
                }
                0x0F => return self.return_from_function(),
                _ => self.execute_opcode(module, opcode)?,
            }
        }
    }

    fn record_execution(&self, units: u64) -> Result<()> {
        let Some(instance) = self.instance.as_ref() else {
            return Ok(());
        };

        instance
            .lock()
            .map_err(poisoned_lock)?
            .record_execution(units)
    }

    fn execute_opcode(&mut self, module: &Module, opcode: u8) -> Result<()> {
        match opcode {
            0x00 => Err(WasmError::Trap(TrapCode::Unreachable)),
            0x01 => Ok(()),
            0x02 => self.enter_block(module, FrameKind::Block),
            0x03 => self.enter_block(module, FrameKind::Loop),
            0x04 => self.enter_if(module),
            0x0C => {
                let depth = self.read_var_u32_immediate()?;
                self.branch(depth).map(|_| ())
            }
            0x0D => {
                let depth = self.read_var_u32_immediate()?;
                let condition = self.operand_stack.pop_i32()?;
                if condition != 0 {
                    self.branch(depth).map(|_| ())
                } else {
                    Ok(())
                }
            }
            0x0E => {
                let count = self.read_var_u32_immediate()? as usize;
                let mut labels = Vec::with_capacity(count);
                for _ in 0..count {
                    labels.push(self.read_var_u32_immediate()?);
                }
                let default = self.read_var_u32_immediate()?;
                let selector = self.operand_stack.pop_i32()? as usize;
                let depth = labels.get(selector).copied().unwrap_or(default);
                self.branch(depth).map(|_| ())
            }
            0x10 => {
                let func_idx = self.read_var_u32_immediate()?;
                self.call_function(module, func_idx)
            }
            0x11 => {
                let type_idx = self.read_var_u32_immediate()?;
                let table_idx = self.read_var_u32_immediate()?;
                self.call_indirect(module, type_idx, table_idx)
            }
            0x1A => {
                self.operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
                Ok(())
            }
            0x1B => self.select_value(),
            0x20 => {
                let idx = self.read_var_u32_immediate()? as usize;
                let value = self
                    .current_frame()?
                    .locals
                    .get(idx)
                    .copied()
                    .ok_or_else(|| WasmError::Runtime(format!("local {} out of bounds", idx)))?;
                self.operand_stack.push(value)
            }
            0x21 => {
                let idx = self.read_var_u32_immediate()? as usize;
                let value = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                let frame = self.current_frame_mut()?;
                let local = frame
                    .locals
                    .get_mut(idx)
                    .ok_or_else(|| WasmError::Runtime(format!("local {} out of bounds", idx)))?;
                *local = value;
                self.locals = frame.locals.clone();
                Ok(())
            }
            0x22 => {
                let idx = self.read_var_u32_immediate()? as usize;
                let value = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                let frame = self.current_frame_mut()?;
                let local = frame
                    .locals
                    .get_mut(idx)
                    .ok_or_else(|| WasmError::Runtime(format!("local {} out of bounds", idx)))?;
                *local = value;
                self.locals = frame.locals.clone();
                self.operand_stack.push(value)
            }
            0x23 => {
                let idx = self.read_var_u32_immediate()?;
                let instance = self.instance_ref()?;
                let instance = instance.lock().map_err(poisoned_lock)?;
                let global = instance
                    .global(idx)
                    .ok_or_else(|| WasmError::Runtime(format!("global {} out of bounds", idx)))?;
                let value = global.lock().map_err(poisoned_lock)?.get();
                drop(instance);
                self.operand_stack.push(value)
            }
            0x24 => {
                let idx = self.read_var_u32_immediate()?;
                let value = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                let instance = self.instance_ref()?;
                let mut instance = instance.lock().map_err(poisoned_lock)?;
                let global = instance
                    .global_mut(idx)
                    .ok_or_else(|| WasmError::Runtime(format!("global {} out of bounds", idx)))?;
                global.lock().map_err(poisoned_lock)?.set(value)
            }
            0x25 => {
                let table_idx = self.read_var_u32_immediate()?;
                let elem_idx = self.operand_stack.pop_i32()? as u32;
                let instance = self.instance_ref()?;
                let instance = instance.lock().map_err(poisoned_lock)?;
                let table = instance.table(table_idx).ok_or_else(|| {
                    WasmError::Runtime(format!("table {} out of bounds", table_idx))
                })?;
                let value = table
                    .lock()
                    .map_err(poisoned_lock)?
                    .get(elem_idx)
                    .ok_or(WasmError::Trap(TrapCode::TableOutOfBounds))?;
                drop(instance);
                self.operand_stack.push(value)
            }
            0x26 => {
                let table_idx = self.read_var_u32_immediate()?;
                let value = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".into()))?;
                let elem_idx = self.operand_stack.pop_i32()? as u32;

                let instance = self.instance_ref()?;
                let mut instance = instance.lock().map_err(poisoned_lock)?;
                let table = instance.table_mut(table_idx).ok_or_else(|| {
                    WasmError::Runtime(format!("table {} out of bounds", table_idx))
                })?;
                table.lock().map_err(poisoned_lock)?.set(elem_idx, value)
            }
            0x28 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I32(self.read_memory_i32(address)?))
            }
            0x29 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_i64(address)?))
            }
            0x2A => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::F32(self.read_memory_f32(address)?))
            }
            0x2B => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::F64(self.read_memory_f64(address)?))
            }
            0x2C => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I32(self.read_memory_u8(address)? as i8 as i32))
            }
            0x2D => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I32(self.read_memory_u8(address)? as i32))
            }
            0x2E => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I32(self.read_memory_u16(address)? as i16 as i32))
            }
            0x2F => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I32(self.read_memory_u16(address)? as i32))
            }
            0x30 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u8(address)? as i8 as i64))
            }
            0x31 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u8(address)? as i64))
            }
            0x32 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u16(address)? as i16 as i64))
            }
            0x33 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u16(address)? as i64))
            }
            0x34 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u32(address)? as i32 as i64))
            }
            0x35 => {
                let offset = self.read_memarg()?;
                let address = self.effective_address(offset)?;
                self.operand_stack
                    .push(WasmValue::I64(self.read_memory_u32(address)? as i64))
            }
            0x36 => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i32()?;
                let address = self.effective_address(offset)?;
                self.write_memory_i32(address, value)
            }
            0x37 => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i64()?;
                let address = self.effective_address(offset)?;
                self.write_memory_i64(address, value)
            }
            0x38 => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_f32()?;
                let address = self.effective_address(offset)?;
                self.write_memory_f32(address, value)
            }
            0x39 => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_f64()?;
                let address = self.effective_address(offset)?;
                self.write_memory_f64(address, value)
            }
            0x3A => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i32()? as u8;
                let address = self.effective_address(offset)?;
                self.write_memory_u8(address, value)
            }
            0x3B => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i32()? as u16;
                let address = self.effective_address(offset)?;
                self.write_memory_u16(address, value)
            }
            0x3C => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i64()? as u8;
                let address = self.effective_address(offset)?;
                self.write_memory_u8(address, value)
            }
            0x3D => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i64()? as u16;
                let address = self.effective_address(offset)?;
                self.write_memory_u16(address, value)
            }
            0x3E => {
                let offset = self.read_memarg()?;
                let value = self.operand_stack.pop_i64()? as u32;
                let address = self.effective_address(offset)?;
                self.write_memory_u32(address, value)
            }
            0x3F => {
                self.expect_zero_immediate("memory.size")?;
                let instance = self.instance_ref()?;
                let instance = instance.lock().map_err(poisoned_lock)?;
                let memory = instance
                    .memory(0)
                    .ok_or_else(|| WasmError::Runtime("no memory".to_string()))?;
                let size = memory.lock().map_err(poisoned_lock)?.size() as i32;
                drop(instance);
                self.operand_stack.push(WasmValue::I32(size))
            }
            0x40 => {
                self.expect_zero_immediate("memory.grow")?;
                let pages = self.operand_stack.pop_i32()?;
                let instance = self.instance_ref()?;
                let mut instance = instance.lock().map_err(poisoned_lock)?;
                let result = WasmValue::I32(instance.memory_grow_wasm(0, pages)?);
                drop(instance);
                self.operand_stack.push(result)
            }
            0x41 => {
                let value = self.read_var_i32_immediate()?;
                self.operand_stack.push(WasmValue::I32(value))
            }
            0x42 => {
                let value = self.read_var_i64_immediate()?;
                self.operand_stack.push(WasmValue::I64(value))
            }
            0x43 => {
                let value = self.read_fixed_u32_immediate()?;
                self.operand_stack
                    .push(WasmValue::F32(f32::from_bits(value)))
            }
            0x44 => {
                let value = self.read_fixed_u64_immediate()?;
                self.operand_stack
                    .push(WasmValue::F64(f64::from_bits(value)))
            }
            0x45 => {
                let value = self.operand_stack.pop_i32()? == 0;
                self.push_bool(value)
            }
            0x46 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a == b)
            }
            0x47 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a != b)
            }
            0x48 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a < b)
            }
            0x49 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                self.push_bool(a < b)
            }
            0x4A => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a > b)
            }
            0x4B => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                self.push_bool(a > b)
            }
            0x4C => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a <= b)
            }
            0x4D => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                self.push_bool(a <= b)
            }
            0x4E => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.push_bool(a >= b)
            }
            0x4F => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                self.push_bool(a >= b)
            }
            0x6A => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_add(b)))
            }
            0x6B => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_sub(b)))
            }
            0x6C => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_mul(b)))
            }
            0x6D => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                if a == i32::MIN && b == -1 {
                    return Err(WasmError::Trap(TrapCode::IntegerOverflow));
                }
                self.operand_stack.push(WasmValue::I32(a / b))
            }
            0x6E => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push(WasmValue::I32((a / b) as i32))
            }
            0x6F => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                if a == i32::MIN && b == -1 {
                    self.operand_stack.push(WasmValue::I32(0))
                } else {
                    self.operand_stack.push(WasmValue::I32(a % b))
                }
            }
            0x70 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push(WasmValue::I32((a % b) as i32))
            }
            0x71 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a & b))
            }
            0x72 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a | b))
            }
            0x73 => {
                let b = self.operand_stack.pop_i32()?;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a ^ b))
            }
            0x74 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_shl(b)))
            }
            0x75 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()?;
                self.operand_stack.push(WasmValue::I32(a.wrapping_shr(b)))
            }
            0x76 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i32()? as u32;
                self.operand_stack
                    .push(WasmValue::I32(a.wrapping_shr(b) as i32))
            }
            0x79 => {
                let value = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push(WasmValue::I64(value.leading_zeros() as i64))
            }
            0x7A => {
                let value = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push(WasmValue::I64(value.trailing_zeros() as i64))
            }
            0x7B => {
                let value = self.operand_stack.pop_i64()?;
                self.operand_stack
                    .push(WasmValue::I64(value.count_ones() as i64))
            }
            0x7C => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a.wrapping_add(b)))
            }
            0x7D => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a.wrapping_sub(b)))
            }
            0x7E => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a.wrapping_mul(b)))
            }
            0x7F => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                if a == i64::MIN && b == -1 {
                    return Err(WasmError::Trap(TrapCode::IntegerOverflow));
                }
                self.operand_stack.push(WasmValue::I64(a / b))
            }
            0x80 => {
                let b = self.operand_stack.pop_i64()? as u64;
                let a = self.operand_stack.pop_i64()? as u64;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push(WasmValue::I64((a / b) as i64))
            }
            0x81 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                if a == i64::MIN && b == -1 {
                    self.operand_stack.push(WasmValue::I64(0))
                } else {
                    self.operand_stack.push(WasmValue::I64(a % b))
                }
            }
            0x82 => {
                let b = self.operand_stack.pop_i64()? as u64;
                let a = self.operand_stack.pop_i64()? as u64;
                if b == 0 {
                    return Err(WasmError::Trap(TrapCode::IntegerDivisionByZero));
                }
                self.operand_stack.push(WasmValue::I64((a % b) as i64))
            }
            0x83 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a & b))
            }
            0x84 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a | b))
            }
            0x85 => {
                let b = self.operand_stack.pop_i64()?;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a ^ b))
            }
            0x86 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a.wrapping_shl(b)))
            }
            0x87 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i64()?;
                self.operand_stack.push(WasmValue::I64(a.wrapping_shr(b)))
            }
            0x88 => {
                let b = self.operand_stack.pop_i32()? as u32;
                let a = self.operand_stack.pop_i64()? as u64;
                self.operand_stack
                    .push(WasmValue::I64(a.wrapping_shr(b) as i64))
            }
            0x92 => {
                let b = self.operand_stack.pop_f32()?;
                let a = self.operand_stack.pop_f32()?;
                self.operand_stack.push(WasmValue::F32(a.copysign(b)))
            }
            0xA6 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push(WasmValue::F64(a + b))
            }
            0xA7 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push(WasmValue::F64(a - b))
            }
            0xA8 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push(WasmValue::F64(a * b))
            }
            0xA9 => {
                let b = self.operand_stack.pop_f64()?;
                let a = self.operand_stack.pop_f64()?;
                self.operand_stack.push(WasmValue::F64(a / b))
            }
            0xD0 => {
                let ref_type = self.read_u8_immediate()?;
                match ref_type {
                    0x70 => self
                        .operand_stack
                        .push(WasmValue::NullRef(RefType::FuncRef)),
                    0x6F => self
                        .operand_stack
                        .push(WasmValue::NullRef(RefType::ExternRef)),
                    _ => Err(WasmError::Runtime(format!(
                        "invalid ref.null type: {:02x}",
                        ref_type
                    ))),
                }
            }
            0xD1 => {
                let value = self
                    .operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
                self.push_bool(matches!(value, WasmValue::NullRef(_)))
            }
            0xD2 => {
                let func_idx = self.read_var_u32_immediate()?;
                self.operand_stack.push(WasmValue::FuncRef(func_idx))
            }
            0xFE => {
                let subopcode = self.read_u8_immediate()?;
                self.execute_atomic_opcode(module, subopcode);
                Ok(())
            }
            _ => Err(WasmError::Runtime(format!(
                "unsupported opcode: {:02x}",
                opcode
            ))),
        }
    }

    fn execute_atomic_opcode(&mut self, _module: &Module, subopcode: u8) {
        match subopcode {
            0x00 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_i32(address).unwrap();
                self.operand_stack.push(WasmValue::I32(value)).unwrap();
            }
            0x01 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_i64(address).unwrap();
                self.operand_stack.push(WasmValue::I64(value)).unwrap();
            }
            0x02 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u8(address).unwrap() as i32;
                self.operand_stack.push(WasmValue::I32(value)).unwrap();
            }
            0x03 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u16(address).unwrap() as i32;
                self.operand_stack.push(WasmValue::I32(value)).unwrap();
            }
            0x04 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u8(address).unwrap() as i8 as i64;
                self.operand_stack.push(WasmValue::I64(value)).unwrap();
            }
            0x05 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u8(address).unwrap() as i64;
                self.operand_stack.push(WasmValue::I64(value)).unwrap();
            }
            0x06 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u16(address).unwrap() as i16 as i64;
                self.operand_stack.push(WasmValue::I64(value)).unwrap();
            }
            0x07 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let value = self.read_memory_u32(address).unwrap() as i64;
                self.operand_stack.push(WasmValue::I64(value)).unwrap();
            }
            0x0A => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i32().unwrap();
                let address = self.effective_address(offset).unwrap();
                self.write_memory_i32(address, value).unwrap();
            }
            0x0B => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i64().unwrap();
                let address = self.effective_address(offset).unwrap();
                self.write_memory_i64(address, value).unwrap();
            }
            0x0C => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i32().unwrap() as u8;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u8(address, value).unwrap();
            }
            0x0D => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i32().unwrap() as u16;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u16(address, value).unwrap();
            }
            0x0E => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i64().unwrap() as u8;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u8(address, value).unwrap();
            }
            0x0F => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i64().unwrap() as u16;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u16(address, value).unwrap();
            }
            0x10 => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i64().unwrap() as u32;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u32(address, value).unwrap();
            }
            0x11 => {
                let offset = self.read_memarg().unwrap();
                let value = self.operand_stack.pop_i64().unwrap() as u64;
                let address = self.effective_address(offset).unwrap();
                self.write_memory_u64(address, value).unwrap();
            }
            0x12 => {
                let offset = self.read_memarg().unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let address = self.effective_address(offset).unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, lhs.wrapping_add(rhs))
                    .unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x13 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, lhs.wrapping_add(rhs))
                    .unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x14 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, lhs.wrapping_sub(rhs))
                    .unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x15 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, lhs.wrapping_sub(rhs))
                    .unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x16 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, lhs & rhs).unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x17 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, lhs & rhs).unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x18 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, lhs | rhs).unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x19 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, lhs | rhs).unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x1A => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, lhs ^ rhs).unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x1B => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, lhs ^ rhs).unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x1C => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i32().unwrap();
                let lhs = self.read_memory_i32(address).unwrap();
                self.write_memory_i32(address, rhs).unwrap();
                self.operand_stack.push(WasmValue::I32(lhs)).unwrap();
            }
            0x1D => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let rhs = self.operand_stack.pop_i64().unwrap();
                let lhs = self.read_memory_i64(address).unwrap();
                self.write_memory_i64(address, rhs).unwrap();
                self.operand_stack.push(WasmValue::I64(lhs)).unwrap();
            }
            0x1E => {
                let offset = self.read_memarg().unwrap();
                let new = self.operand_stack.pop_i32().unwrap();
                let expected = self.operand_stack.pop_i32().unwrap();
                let address = self.effective_address(offset).unwrap();
                let old = self.read_memory_i32(address).unwrap();
                if old == expected {
                    self.write_memory_i32(address, new).unwrap();
                }
                self.operand_stack.push(WasmValue::I32(old)).unwrap();
            }
            0x1F => {
                let offset = self.read_memarg().unwrap();
                let new = self.operand_stack.pop_i64().unwrap();
                let expected = self.operand_stack.pop_i64().unwrap();
                let address = self.effective_address(offset).unwrap();
                let old = self.read_memory_i64(address).unwrap();
                if old == expected {
                    self.write_memory_i64(address, new).unwrap();
                }
                self.operand_stack.push(WasmValue::I64(old)).unwrap();
            }
            0x37 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let n = self.operand_stack.pop_i32().unwrap() as u32;
                let instance = self.instance_ref().unwrap();
                let instance = instance.lock().unwrap();
                let memory = instance.memory(0).unwrap();
                let notified = memory.lock().unwrap().notify(address, n).unwrap() as i32;
                drop(instance);
                self.operand_stack.push(WasmValue::I32(notified)).unwrap();
            }
            0x38 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let expected = self.operand_stack.pop_i64().unwrap();
                let timeout = self.operand_stack.pop_i64().unwrap();
                let instance = self.instance_ref().unwrap();
                let instance = instance.lock().unwrap();
                let result = instance.wait32(address, expected, timeout).unwrap();
                drop(instance);
                self.operand_stack.push(WasmValue::I32(result)).unwrap();
            }
            0x39 => {
                let offset = self.read_memarg().unwrap();
                let address = self.effective_address(offset).unwrap();
                let expected = self.operand_stack.pop_i64().unwrap();
                let timeout = self.operand_stack.pop_i64().unwrap();
                let instance = self.instance_ref().unwrap();
                let instance = instance.lock().unwrap();
                let result = instance.wait64(address, expected, timeout).unwrap();
                drop(instance);
                self.operand_stack.push(WasmValue::I32(result)).unwrap();
            }
            0xFF => {
                std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
            }
            _ => {}
        }
    }

    fn call_function(&mut self, module: &Module, func_idx: u32) -> Result<()> {
        let func_type = module.func_type(func_idx).ok_or_else(|| {
            WasmError::Validation(format!("function type not found for func {}", func_idx))
        })?;

        let args = self.pop_args(func_type)?;
        let import_func_count = module
            .imports
            .iter()
            .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
            .count() as u32;

        if func_idx < import_func_count {
            let instance = self.instance_ref()?;
            let mut instance = instance.lock().map_err(poisoned_lock)?;
            let outcome = instance.call_with_suspension(func_idx, &args)?;
            drop(instance);

            match outcome {
                HostCallOutcome::Complete(results) => {
                    for value in results {
                        self.operand_stack.push(value)?;
                    }
                }
                HostCallOutcome::Pending { pending_work } => {
                    let resume_pc = self
                        .control_stack
                        .last()
                        .map(|frame| frame.position)
                        .unwrap_or(0);
                    self.suspend_hostcall_state_at(
                        pending_work,
                        func_type.results.clone(),
                        resume_pc,
                    )
                    .map_err(|e| {
                        WasmError::Runtime(format!("hostcall suspension failed: {}", e))
                    })?;
                    return Err(WasmError::Suspended(SuspensionKind::HostcallPending));
                }
            }
            return Ok(());
        }

        let mut frame = self.build_frame(module, func_idx, &args)?;
        frame.height = self.operand_stack.len();
        self.locals = frame.locals.clone();
        self.control_stack.push(frame);
        Ok(())
    }

    fn call_indirect(&mut self, module: &Module, type_idx: u32, table_idx: u32) -> Result<()> {
        let expected_type = module
            .type_at(type_idx)
            .ok_or_else(|| WasmError::Validation(format!("type {} not found", type_idx)))?;
        let elem_idx = self.operand_stack.pop_i32()? as u32;

        let target_func_idx = {
            let instance = self.instance_ref()?;
            let instance = instance.lock().map_err(poisoned_lock)?;
            let table = instance
                .table(table_idx)
                .ok_or_else(|| WasmError::Runtime(format!("table {} out of bounds", table_idx)))?;
            match table
                .lock()
                .map_err(poisoned_lock)?
                .get(elem_idx)
                .ok_or(WasmError::Trap(TrapCode::TableOutOfBounds))?
            {
                WasmValue::FuncRef(func_idx) => func_idx,
                WasmValue::NullRef(RefType::FuncRef) => {
                    return Err(WasmError::Trap(TrapCode::CallIndirectNull));
                }
                _ => {
                    return Err(WasmError::Runtime(
                        "table element is not a funcref".to_string(),
                    ));
                }
            }
        };

        let target_type = module.func_type(target_func_idx).ok_or_else(|| {
            WasmError::Validation(format!(
                "function type not found for func {}",
                target_func_idx
            ))
        })?;
        if target_type != expected_type {
            return Err(WasmError::Trap(TrapCode::IndirectCallTypeMismatch));
        }

        self.call_function(module, target_func_idx)
    }

    fn build_frame(
        &self,
        module: &Module,
        func_idx: u32,
        args: &[WasmValue],
    ) -> Result<ControlFrame> {
        let import_func_count = module
            .imports
            .iter()
            .filter(|import| matches!(import.kind, crate::runtime::ImportKind::Func(_)))
            .count() as u32;
        if func_idx < import_func_count {
            return Err(WasmError::Runtime(
                "imported functions must be invoked through an instance".to_string(),
            ));
        }

        let local_idx = func_idx - import_func_count;
        let func = module
            .defined_func_at(local_idx)
            .ok_or_else(|| WasmError::Runtime(format!("function {} not found", func_idx)))?;
        let func_type = module
            .type_at(func.type_idx)
            .ok_or_else(|| WasmError::Validation(format!("type {} not found", func.type_idx)))?;

        if args.len() != func_type.params.len() {
            return Err(WasmError::Runtime(format!(
                "function {} expects {} args, got {}",
                func_idx,
                func_type.params.len(),
                args.len()
            )));
        }
        for (index, (arg, expected_type)) in args.iter().zip(func_type.params.iter()).enumerate() {
            if arg.val_type() != *expected_type {
                return Err(WasmError::Runtime(format!(
                    "function {} argument {} type mismatch: expected {:?}, got {:?}",
                    func_idx,
                    index,
                    expected_type,
                    arg.val_type()
                )));
            }
        }

        let mut locals = args.to_vec();
        for local in &func.locals {
            for _ in 0..local.count {
                locals.push(default_value(local.type_));
            }
        }

        Ok(ControlFrame::new(
            FrameKind::Function,
            func_type.params.len() as u32,
            func_type.results.len() as u32,
            func_type.results.len() as u32,
            func.body.clone(),
            locals,
        ))
    }

    fn enter_block(&mut self, module: &Module, kind: FrameKind) -> Result<()> {
        let signature = self.read_block_signature(module)?;
        let split = {
            let frame = self.current_frame()?;
            self.scan_control_frame(&frame.code, frame.position, false)?
        };
        let locals = self.locals.clone();
        {
            let frame = self.current_frame_mut()?;
            frame.position = split.after_end;
        }

        let label_arity = match kind {
            FrameKind::Loop => signature.param_count,
            FrameKind::Block => signature.result_count,
            FrameKind::Function => signature.result_count,
        };
        let mut block = ControlFrame::new(
            kind,
            signature.param_count as u32,
            signature.result_count as u32,
            label_arity as u32,
            split.then_body,
            locals,
        );
        block.height = self
            .operand_stack
            .len()
            .saturating_sub(signature.param_count);
        self.control_stack.push(block);
        Ok(())
    }

    fn enter_if(&mut self, module: &Module) -> Result<()> {
        let signature = self.read_block_signature(module)?;
        let condition = self.operand_stack.pop_i32()?;
        let split = {
            let frame = self.current_frame()?;
            self.scan_control_frame(&frame.code, frame.position, true)?
        };
        let locals = self.locals.clone();
        {
            let frame = self.current_frame_mut()?;
            frame.position = split.after_end;
        }

        let selected = if condition == 0 {
            split.else_body.unwrap_or_default()
        } else {
            split.then_body
        };
        let mut block = ControlFrame::new(
            FrameKind::Block,
            signature.param_count as u32,
            signature.result_count as u32,
            signature.result_count as u32,
            selected,
            locals,
        );
        block.height = self
            .operand_stack
            .len()
            .saturating_sub(signature.param_count);
        self.control_stack.push(block);
        Ok(())
    }

    fn finish_frame(&mut self) -> Result<Option<Vec<WasmValue>>> {
        let frame = self
            .control_stack
            .pop_frame()
            .ok_or_else(|| WasmError::Runtime("no frame to finish".to_string()))?;

        let mut results = Vec::with_capacity(frame.arity);
        for _ in 0..frame.arity {
            let value = self
                .operand_stack
                .pop()
                .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
            results.push(value);
        }
        results.reverse();

        self.operand_stack.truncate(frame.height);

        if let Some(parent) = self.control_stack.last_mut() {
            parent.locals = frame.locals.clone();
            for value in &results {
                self.operand_stack.push(*value)?;
            }
            self.locals = parent.locals.clone();
            Ok(None)
        } else {
            self.locals.clear();
            Ok(Some(results))
        }
    }

    fn return_from_function(&mut self) -> Result<Vec<WasmValue>> {
        let function_frame = self
            .control_stack
            .frames()
            .iter()
            .rev()
            .find(|frame| matches!(frame.kind, FrameKind::Function))
            .cloned()
            .ok_or_else(|| WasmError::Runtime("no function frame to return from".to_string()))?;

        let mut results = Vec::with_capacity(function_frame.arity);
        for _ in 0..function_frame.arity {
            let value = self
                .operand_stack
                .pop()
                .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
            results.push(value);
        }
        results.reverse();
        self.operand_stack.truncate(function_frame.height);
        self.control_stack.clear();
        self.locals.clear();
        Ok(results)
    }

    fn branch(&mut self, depth: u32) -> Result<Option<Vec<WasmValue>>> {
        let len = self.control_stack.len();
        let target_index = len
            .checked_sub(depth as usize + 1)
            .ok_or_else(|| WasmError::Runtime(format!("invalid branch depth {}", depth)))?;
        let target = self
            .control_stack
            .get(target_index)
            .cloned()
            .ok_or_else(|| WasmError::Runtime(format!("invalid branch depth {}", depth)))?;

        let label_arity = target.label_arity;
        let mut values = Vec::with_capacity(label_arity);
        for _ in 0..label_arity {
            values.push(
                self.operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?,
            );
        }
        values.reverse();
        self.operand_stack.truncate(target.height);
        self.control_stack.truncate(target_index + 1);

        match target.kind {
            FrameKind::Loop => {
                let loop_frame = self
                    .control_stack
                    .get_mut(target_index)
                    .ok_or_else(|| WasmError::Runtime("loop frame missing".to_string()))?;
                loop_frame.position = 0;
                loop_frame.locals = self.locals.clone();
                for value in &values {
                    self.operand_stack.push(*value)?;
                }
                self.locals = loop_frame.locals.clone();
                Ok(None)
            }
            FrameKind::Block | FrameKind::Function => {
                let target_frame = self
                    .control_stack
                    .get_mut(target_index)
                    .ok_or_else(|| WasmError::Runtime("branch target missing".to_string()))?;
                target_frame.position = target_frame.code.len();
                target_frame.locals = self.locals.clone();
                for value in &values {
                    self.operand_stack.push(*value)?;
                }
                self.locals = target_frame.locals.clone();
                Ok(None)
            }
        }
    }

    fn pop_args(&mut self, func_type: &FunctionType) -> Result<Vec<WasmValue>> {
        let mut args = Vec::with_capacity(func_type.params.len());
        for _ in 0..func_type.params.len() {
            args.push(
                self.operand_stack
                    .pop()
                    .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?,
            );
        }
        args.reverse();
        Ok(args)
    }

    fn select_value(&mut self) -> Result<()> {
        let condition = self.operand_stack.pop_i32()?;
        let rhs = self
            .operand_stack
            .pop()
            .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
        let lhs = self
            .operand_stack
            .pop()
            .ok_or_else(|| WasmError::Runtime("stack underflow".to_string()))?;
        self.operand_stack
            .push(if condition == 0 { rhs } else { lhs })
    }

    fn push_bool(&mut self, value: bool) -> Result<()> {
        self.operand_stack
            .push(WasmValue::I32(if value { 1 } else { 0 }))
    }

    fn read_memarg(&mut self) -> Result<u32> {
        let _align = self.read_var_u32_immediate()?;
        self.read_var_u32_immediate()
    }

    fn effective_address(&mut self, offset: u32) -> Result<u32> {
        (self.operand_stack.pop_i32()? as u32)
            .checked_add(offset)
            .ok_or(WasmError::Trap(TrapCode::MemoryOutOfBounds))
    }

    fn with_memory<T>(&self, f: impl FnOnce(&crate::memory::Memory) -> Result<T>) -> Result<T> {
        let memory = {
            let instance = self.instance_ref()?;
            let instance = instance.lock().map_err(poisoned_lock)?;
            instance
                .memory(0)
                .cloned()
                .ok_or_else(|| WasmError::Runtime("no memory".to_string()))?
        };
        let memory = memory.lock().map_err(poisoned_lock)?;
        f(&memory)
    }

    fn with_memory_mut<T>(
        &self,
        f: impl FnOnce(&mut crate::memory::Memory) -> Result<T>,
    ) -> Result<T> {
        let memory = {
            let instance = self.instance_ref()?;
            let instance = instance.lock().map_err(poisoned_lock)?;
            instance
                .memory(0)
                .cloned()
                .ok_or_else(|| WasmError::Runtime("no memory".to_string()))?
        };
        let mut memory = memory.lock().map_err(poisoned_lock)?;
        f(&mut memory)
    }

    fn read_memory_u8(&self, address: u32) -> Result<u8> {
        self.with_memory(|memory| memory.read_u8(address))
    }

    fn read_memory_u16(&self, address: u32) -> Result<u16> {
        self.with_memory(|memory| {
            let mut bytes = [0u8; 2];
            memory.read(address, &mut bytes)?;
            Ok(u16::from_le_bytes(bytes))
        })
    }

    fn read_memory_u32(&self, address: u32) -> Result<u32> {
        self.with_memory(|memory| memory.read_u32(address))
    }

    fn read_memory_i32(&self, address: u32) -> Result<i32> {
        self.with_memory(|memory| memory.read_i32(address))
    }

    fn read_memory_i64(&self, address: u32) -> Result<i64> {
        self.with_memory(|memory| memory.read_i64(address))
    }

    fn read_memory_f32(&self, address: u32) -> Result<f32> {
        self.with_memory(|memory| memory.read_f32(address))
    }

    fn read_memory_f64(&self, address: u32) -> Result<f64> {
        self.with_memory(|memory| memory.read_f64(address))
    }

    fn write_memory_u8(&self, address: u32, value: u8) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_u8(address, value))
    }

    fn write_memory_u16(&self, address: u32, value: u16) -> Result<()> {
        self.with_memory_mut(|memory| memory.write(address, &value.to_le_bytes()))
    }

    fn write_memory_u32(&self, address: u32, value: u32) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_u32(address, value))
    }

    fn write_memory_i32(&self, address: u32, value: i32) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_i32(address, value))
    }

    fn write_memory_i64(&self, address: u32, value: i64) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_i64(address, value))
    }

    fn write_memory_u64(&self, address: u32, value: u64) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_u64(address, value))
    }

    fn write_memory_f32(&self, address: u32, value: f32) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_f32(address, value))
    }

    fn write_memory_f64(&self, address: u32, value: f64) -> Result<()> {
        self.with_memory_mut(|memory| memory.write_f64(address, value))
    }

    fn expect_zero_immediate(&mut self, instruction: &str) -> Result<()> {
        let reserved = self.read_u8_immediate()?;
        if reserved != 0 {
            return Err(WasmError::Runtime(format!(
                "{} expects a zero reserved byte",
                instruction
            )));
        }
        Ok(())
    }

    fn read_block_signature(&mut self, module: &Module) -> Result<BlockSignature> {
        let marker = self.read_u8_immediate()?;
        match marker {
            0x40 => Ok(BlockSignature {
                param_count: 0,
                result_count: 0,
            }),
            0x7F | 0x7E | 0x7D | 0x7C | 0x70 | 0x6F => Ok(BlockSignature {
                param_count: 0,
                result_count: 1,
            }),
            byte => {
                let type_idx = self.read_signed_leb_continuation(byte)?;
                if type_idx < 0 {
                    return Err(WasmError::Validation(format!(
                        "invalid block type index {}",
                        type_idx
                    )));
                }
                let type_ = module
                    .type_at(type_idx as u32)
                    .ok_or_else(|| WasmError::Validation(format!("type {} not found", type_idx)))?;
                Ok(BlockSignature {
                    param_count: type_.params.len(),
                    result_count: type_.results.len(),
                })
            }
        }
    }

    fn scan_control_frame(
        &self,
        code: &[u8],
        start: usize,
        allow_else: bool,
    ) -> Result<ControlSplit> {
        let mut cursor = start;
        let mut depth = 1usize;
        let mut else_at = None;

        while cursor < code.len() {
            let opcode = code[cursor];
            cursor += 1;
            match opcode {
                0x02..=0x04 => {
                    Self::skip_block_type(code, &mut cursor)?;
                    depth += 1;
                }
                0x05 if allow_else && depth == 1 => {
                    else_at = Some(cursor - 1);
                }
                0x0B => {
                    depth -= 1;
                    if depth == 0 {
                        let then_end = else_at.unwrap_or(cursor - 1);
                        let then_body = code[start..then_end].to_vec();
                        let else_body =
                            else_at.map(|else_pos| code[else_pos + 1..cursor - 1].to_vec());
                        return Ok(ControlSplit {
                            then_body,
                            else_body,
                            after_end: cursor,
                        });
                    }
                }
                _ => Self::skip_immediates(code, &mut cursor, opcode)?,
            }
        }

        Err(WasmError::Load(
            "unterminated structured control instruction".to_string(),
        ))
    }

    fn skip_block_type(code: &[u8], cursor: &mut usize) -> Result<()> {
        let marker = *code
            .get(*cursor)
            .ok_or_else(|| WasmError::Load("unexpected end of block type".to_string()))?;
        *cursor += 1;
        if !matches!(marker, 0x40 | 0x7F | 0x7E | 0x7D | 0x7C | 0x70 | 0x6F) {
            Self::skip_sleb_tail(code, cursor, marker)?;
        }
        Ok(())
    }

    fn skip_immediates(code: &[u8], cursor: &mut usize, opcode: u8) -> Result<()> {
        match opcode {
            0x0C | 0x0D | 0x10 | 0x20..=0x26 | 0xD2 => Self::skip_uleb(code, cursor),
            0x0E => {
                let count = Self::read_uleb(code, cursor)?;
                for _ in 0..count {
                    Self::skip_uleb(code, cursor)?;
                }
                Self::skip_uleb(code, cursor)
            }
            0x11 => {
                Self::skip_uleb(code, cursor)?;
                Self::skip_uleb(code, cursor)
            }
            0x1C => {
                let count = Self::read_uleb(code, cursor)?;
                for _ in 0..count {
                    Self::skip_bytes(code, cursor, 1)?;
                }
                Ok(())
            }
            0x28..=0x3E => {
                Self::skip_uleb(code, cursor)?;
                Self::skip_uleb(code, cursor)
            }
            0x3F | 0x40 | 0xD0 => Self::skip_bytes(code, cursor, 1),
            0x41 | 0x42 => Self::skip_sleb(code, cursor),
            0x43 => Self::skip_bytes(code, cursor, 4),
            0x44 => Self::skip_bytes(code, cursor, 8),
            0xFC => Err(WasmError::Runtime(
                "unsupported 0xfc prefixed opcode in structured control".to_string(),
            )),
            _ => Ok(()),
        }
    }

    fn skip_uleb(code: &[u8], cursor: &mut usize) -> Result<()> {
        let _ = Self::read_uleb(code, cursor)?;
        Ok(())
    }

    fn read_uleb(code: &[u8], cursor: &mut usize) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0u32;
        loop {
            let byte = *code
                .get(*cursor)
                .ok_or_else(|| WasmError::Load("unexpected end of uleb immediate".to_string()))?;
            *cursor += 1;
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Load("uleb128 overflow".to_string()));
            }
        }
    }

    fn skip_sleb(code: &[u8], cursor: &mut usize) -> Result<()> {
        let first = *code
            .get(*cursor)
            .ok_or_else(|| WasmError::Load("unexpected end of sleb immediate".to_string()))?;
        *cursor += 1;
        Self::skip_sleb_tail(code, cursor, first)
    }

    fn skip_sleb_tail(code: &[u8], cursor: &mut usize, mut byte: u8) -> Result<()> {
        while byte & 0x80 != 0 {
            byte = *code
                .get(*cursor)
                .ok_or_else(|| WasmError::Load("unexpected end of sleb immediate".to_string()))?;
            *cursor += 1;
        }
        Ok(())
    }

    fn skip_bytes(code: &[u8], cursor: &mut usize, len: usize) -> Result<()> {
        if code.len().saturating_sub(*cursor) < len {
            return Err(WasmError::Load("unexpected end of immediate".to_string()));
        }
        *cursor += len;
        Ok(())
    }

    fn read_signed_leb_continuation(&mut self, first: u8) -> Result<i32> {
        let mut result = (first & 0x7F) as i32;
        let mut shift = 7u32;
        let mut byte = first;

        while byte & 0x80 != 0 {
            byte = self.read_u8_immediate()?;
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Load("sleb128 overflow".to_string()));
            }
        }

        if shift < 32 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }

        Ok(result)
    }

    fn current_frame(&self) -> Result<&ControlFrame> {
        self.control_stack
            .last()
            .ok_or_else(|| WasmError::Runtime("no active frame".to_string()))
    }

    fn current_frame_mut(&mut self) -> Result<&mut ControlFrame> {
        self.control_stack
            .last_mut()
            .ok_or_else(|| WasmError::Runtime("no active frame".to_string()))
    }

    fn read_u8_immediate(&mut self) -> Result<u8> {
        let frame = self.current_frame_mut()?;
        if frame.position >= frame.code.len() {
            return Err(WasmError::Load(
                "unexpected end of function body".to_string(),
            ));
        }
        let byte = frame.code[frame.position];
        frame.position += 1;
        Ok(byte)
    }

    fn read_var_u32_immediate(&mut self) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0u32;

        loop {
            let byte = self.read_u8_immediate()?;
            result |= ((byte & 0x7F) as u32) << shift;

            if byte & 0x80 == 0 {
                return Ok(result);
            }

            shift += 7;
            if shift >= 35 {
                return Err(WasmError::Load("uleb128 overflow".to_string()));
            }
        }
    }

    fn read_var_i32_immediate(&mut self) -> Result<i32> {
        let mut result = 0i32;
        let mut shift = 0u32;
        let mut byte;

        loop {
            byte = self.read_u8_immediate()?;
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 35 {
                return Err(WasmError::Load("sleb128 overflow".to_string()));
            }
        }

        if shift < 32 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }

        Ok(result)
    }

    fn read_var_i64_immediate(&mut self) -> Result<i64> {
        let mut result = 0i64;
        let mut shift = 0u32;
        let mut byte;

        loop {
            byte = self.read_u8_immediate()?;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 70 {
                return Err(WasmError::Load("sleb128 overflow".to_string()));
            }
        }

        if shift < 64 && (byte & 0x40) != 0 {
            result |= !0 << shift;
        }

        Ok(result)
    }

    fn read_fixed_u32_immediate(&mut self) -> Result<u32> {
        let mut bytes = [0u8; 4];
        for byte in &mut bytes {
            *byte = self.read_u8_immediate()?;
        }
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_fixed_u64_immediate(&mut self) -> Result<u64> {
        let mut bytes = [0u8; 8];
        for byte in &mut bytes {
            *byte = self.read_u8_immediate()?;
        }
        Ok(u64::from_le_bytes(bytes))
    }

    fn instance_ref(&self) -> Result<&Arc<Mutex<Instance>>> {
        self.instance
            .as_ref()
            .ok_or_else(|| WasmError::Runtime("no instance available".to_string()))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

fn default_value(value_type: ValType) -> WasmValue {
    match value_type {
        ValType::Num(NumType::I32) => WasmValue::I32(0),
        ValType::Num(NumType::I64) => WasmValue::I64(0),
        ValType::Num(NumType::F32) => WasmValue::F32(0.0),
        ValType::Num(NumType::F64) => WasmValue::F64(0.0),
        ValType::Ref(RefType::FuncRef) => WasmValue::NullRef(RefType::FuncRef),
        ValType::Ref(RefType::ExternRef) => WasmValue::NullRef(RefType::ExternRef),
    }
}

fn poisoned_lock<T>(_: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>) -> WasmError {
    WasmError::Runtime("instance lock poisoned".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        Extern, Func, FunctionType, HostCallOutcome, HostFunc, Import, ImportKind, Instance,
        Limits, Local, MemoryType, Module, TableType,
    };
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    #[test]
    fn test_interpreter_creation() {
        let interp = Interpreter::new();
        assert!(interp.operand_stack.is_empty());
        assert!(interp.control_stack.is_empty());
    }

    #[test]
    fn test_i32_operations() {
        let mut interp = Interpreter::new();
        interp.operand_stack.push_unchecked(WasmValue::I32(5));
        interp.operand_stack.push_unchecked(WasmValue::I32(3));
        assert_eq!(interp.operand_stack.pop_i32().unwrap(), 3);
        assert_eq!(interp.operand_stack.pop_i32().unwrap(), 5);
    }

    #[test]
    fn test_bit_operations() {
        let mut interp = Interpreter::new();
        interp.operand_stack.push_unchecked(WasmValue::I32(0b1100));
        interp.operand_stack.push_unchecked(WasmValue::I32(0b1010));
        let b = interp.operand_stack.pop_i32().unwrap();
        let a = interp.operand_stack.pop_i32().unwrap();
        let result = a & b;
        assert_eq!(result, 0b1000);
    }

    #[test]
    fn test_if_else_executes_selected_branch() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x04, 0x7F, 0x41, 0x01, 0x05, 0x41, 0x02, 0x0B, 0x0B,
            ],
        });

        let mut interp = Interpreter::new();
        let results = interp.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(2)]);
    }

    #[test]
    fn test_return_unwinds_nested_blocks() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x02, 0x40, 0x41, 0x07, 0x0F, 0x0B, 0x41, 0x00, 0x0B],
        });

        let mut interp = Interpreter::new();
        let results = interp.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(7)]);
    }

    #[test]
    fn test_loop_with_br_and_br_if() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![Local {
                count: 1,
                type_: ValType::Num(NumType::I32),
            }],
            body: vec![
                0x41, 0x03, 0x21, 0x00, 0x02, 0x40, 0x03, 0x40, 0x20, 0x00, 0x45, 0x0D, 0x01, 0x20,
                0x00, 0x41, 0x01, 0x6B, 0x21, 0x00, 0x0C, 0x00, 0x0B, 0x0B, 0x20, 0x00, 0x0B,
            ],
        });

        let mut interp = Interpreter::new();
        let results = interp.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_br_table_selects_target() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x02, 0x7F, 0x02, 0x7F, 0x41, 0x14, 0x41, 0x01, 0x0E, 0x02, 0x00, 0x01, 0x01, 0x41,
                0x0A, 0x0B, 0x41, 0x1E, 0x0B, 0x0B,
            ],
        });

        let mut interp = Interpreter::new();
        let results = interp.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(20)]);
    }

    #[test]
    fn test_typed_loop_branch_uses_parameter_arity() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![ValType::Num(NumType::I32)], vec![]));
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 1,
            locals: vec![Local {
                count: 1,
                type_: ValType::Num(NumType::I32),
            }],
            body: vec![
                0x41, 0x03, 0x03, 0x00, 0x21, 0x00, 0x20, 0x00, 0x20, 0x00, 0x45, 0x0D, 0x01, 0x41,
                0x01, 0x6B, 0x0C, 0x00, 0x0B, 0x20, 0x00, 0x0B,
            ],
        });

        let mut interp = Interpreter::new();
        let results = interp.execute_function(&module, 0, &[]).unwrap();
        assert_eq!(results, vec![WasmValue::I32(0)]);
    }

    #[test]
    fn test_memory_load_and_store_opcodes() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.memories.push(MemoryType::new(Limits::Min(1)));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x41, 0x2A, 0x36, 0x02, 0x00, 0x41, 0x00, 0x28, 0x02, 0x00, 0x0B,
            ],
        });

        let module = Arc::new(module);
        let instance = Arc::new(Mutex::new(Instance::new(module.clone()).unwrap()));
        let mut interp = Interpreter::with_instance(instance);
        let results = interp.execute_function(&module, 0, &[]).unwrap();

        assert_eq!(results, vec![WasmValue::I32(42)]);
    }

    #[test]
    fn test_table_set_accepts_externref_tables() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module
            .tables
            .push(TableType::new(RefType::ExternRef, Limits::Min(1)));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0xD0, 0x6F, 0x26, 0x00, 0x0B],
        });

        let module = Arc::new(module);
        let instance = Arc::new(Mutex::new(Instance::new(module.clone()).unwrap()));
        let mut interp = Interpreter::with_instance(instance.clone());
        interp.execute_function(&module, 0, &[]).unwrap();

        let table = instance.lock().unwrap().table(0).cloned().unwrap();
        assert_eq!(
            table.lock().unwrap().get(0),
            Some(WasmValue::NullRef(RefType::ExternRef))
        );
    }

    #[test]
    fn test_execute_function_rejects_argument_type_mismatch() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![ValType::Num(NumType::I32)], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x0B],
        });

        let mut interp = Interpreter::new();
        let error = interp
            .execute_function(&module, 0, &[WasmValue::F64(1.0)])
            .unwrap_err();

        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("argument 0 type mismatch"))
        );
    }

    #[test]
    fn test_safepoint_suspend_and_resume() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        let func_body = vec![0x41, 0x01, 0x41, 0x02, 0x6A, 0x0B];
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: func_body,
        });

        let mut interp = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let mut suspensions = 0;
        let mut result = interp.execute_function(&module, 0, &[]);
        let final_result = loop {
            match result {
                Ok(values) => break values,
                Err(WasmError::Suspended(SuspensionKind::Safepoint)) => {
                    suspensions += 1;
                    assert!(interp.is_suspended());

                    let handle = interp
                        .take_suspended_handle()
                        .expect("should have suspended handle");
                    assert!(handle.is_suspended());

                    interp.try_resume(&handle).expect("resume should succeed");
                    assert!(!interp.is_suspended());

                    result = interp.continue_execution(&module);
                }
                Err(error) => panic!("unexpected error: {error:?}"),
            }
        };

        assert!(suspensions >= 2);
        assert_eq!(final_result, vec![WasmValue::I32(3)]);
    }

    #[test]
    fn test_safepoints_rearm_when_loop_returns_to_same_pc() {
        let mut module = Module::new();
        module.types.push(FunctionType::new(vec![], vec![]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x03, 0x40, 0x0C, 0x00, 0x0B, 0x0B],
        });

        let mut interp = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let mut result = interp.execute_function(&module, 0, &[]);
        let mut suspensions = 0;

        while suspensions < 3 {
            match result {
                Err(WasmError::Suspended(SuspensionKind::Safepoint)) => {
                    suspensions += 1;
                    let handle = interp.take_suspended_handle().unwrap();
                    interp.try_resume(&handle).unwrap();
                    result = interp.continue_execution(&module);
                }
                other => panic!("expected repeated safepoints, got {other:?}"),
            }
        }

        assert_eq!(suspensions, 3);
    }

    #[test]
    fn test_continue_without_resume_fails() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x05, 0x0B],
        });

        let mut interp = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let result = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let continue_result = interp.continue_execution(&module);
        assert!(matches!(
            continue_result,
            Err(WasmError::Runtime(msg)) if msg.contains("must be resumed first")
        ));
    }

    #[test]
    fn test_continue_without_execution_fails() {
        let module = Module::new();
        let mut interp = Interpreter::new();

        let result = interp.continue_execution(&module);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message))
                if message.contains("no suspended execution is available to continue")
        ));
    }

    #[test]
    fn test_safepoints_require_configured_suspender() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x05, 0x0B],
        });

        let mut interp = Interpreter::new().with_safepoints(SafepointConfig::new(true));
        let result = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message))
                if message.contains("configured runtime suspender")
        ));
    }

    #[test]
    fn test_wrong_interpreter_resume_fails() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x05, 0x0B],
        });

        let mut interp1 = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let result = interp1.execute_function(&module, 0, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let handle = interp1.take_suspended_handle().unwrap();

        let mut interp2 = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true));

        let resume_result = interp2.try_resume(&handle);
        assert!(matches!(
            resume_result,
            Err(WasmError::Runtime(msg)) if msg.contains("different interpreter")
        ));
    }

    #[test]
    fn test_execute_function_while_suspended_fails() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x05, 0x0B],
        });

        let mut interp = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let first = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            first,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let second = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            second,
            Err(WasmError::Runtime(message))
                if message.contains("cannot start a new execution while suspended")
        ));
    }

    #[test]
    fn test_stale_handle_resume_fails() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x05, 0x0B],
        });

        let mut interp = Interpreter::new()
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(1));

        let first = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            first,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));
        let stale_handle = interp.take_suspended_handle().unwrap();

        interp.try_resume(&stale_handle).unwrap();
        let mut resumed = interp.continue_execution(&module);
        loop {
            match resumed {
                Ok(_) => break,
                Err(WasmError::Suspended(SuspensionKind::Safepoint)) => {
                    let handle = interp.take_suspended_handle().unwrap();
                    interp.try_resume(&handle).unwrap();
                    resumed = interp.continue_execution(&module);
                }
                Err(error) => panic!("unexpected error: {error:?}"),
            }
        }

        let second = interp.execute_function(&module, 0, &[]);
        assert!(matches!(
            second,
            Err(WasmError::Suspended(SuspensionKind::Safepoint))
        ));

        let resume_result = interp.try_resume(&stale_handle);
        assert!(matches!(
            resume_result,
            Err(WasmError::Runtime(msg)) if msg.contains("previous execution epoch")
        ));
    }

    #[test]
    fn test_hostcall_pending_resume() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        assert!(handle.has_pending_hostcall());
        assert!(handle.is_suspended());

        let pending = handle.pending_work();
        assert_eq!(pending, Some(vec![1, 2, 3]));

        let result = handle.resume_after_hostcall(&[WasmValue::I32(7)]);
        assert!(result.is_ok());

        let state = result.unwrap();
        if let crate::runtime::SuspensionState::Interpreter(state) = state {
            assert_eq!(state.pc, 10);
            assert_eq!(state.locals, vec![]);
            let mut operand_stack = state.operand_stack;
            assert_eq!(operand_stack.pop(), Some(WasmValue::I32(7)));
        } else {
            panic!("expected interpreter state");
        }
    }

    #[test]
    fn test_try_resume_rejects_pending_hostcall_without_results() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 0;
        interp.needs_resume = true;

        let result = interp.try_resume(&handle);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("requires completion results")
        ));
    }

    #[test]
    fn test_resume_hostcall_rejects_wrong_result_arity() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 0;
        interp.needs_resume = true;
        interp.execution_thread = Some(std::thread::current().id());

        let result = interp.resume_hostcall(&handle, &[]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("result count mismatch")
        ));
    }

    #[test]
    fn test_resume_hostcall_rejects_wrong_result_type() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 0;
        interp.needs_resume = true;
        interp.execution_thread = Some(std::thread::current().id());

        let result = interp.resume_hostcall(&handle, &[WasmValue::I64(7)]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("type mismatch")
        ));
    }

    #[test]
    fn test_resume_hostcall_rejects_stale_epoch() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 1;
        interp.needs_resume = true;
        interp.execution_thread = Some(std::thread::current().id());

        let result = interp.resume_hostcall(&handle, &[WasmValue::I32(7)]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("previous execution epoch")
        ));
    }

    #[test]
    fn test_cross_thread_hostcall_resume_is_rejected() {
        let suspender = RuntimeSuspender::new();
        let state = crate::runtime::InterpreterState::capture(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let handle = suspender.suspend_with_pending_hostcall(
            vec![1, 2, 3],
            vec![ValType::Num(NumType::I32)],
            state,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 0;
        interp.needs_resume = true;
        let other_thread_id = std::thread::spawn(|| std::thread::current().id())
            .join()
            .unwrap();
        interp.execution_thread = Some(other_thread_id);

        let result = interp.resume_hostcall(&handle, &[WasmValue::I32(7)]);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("cross-thread interpreter resume")
        ));
    }

    #[test]
    fn test_cross_thread_interpreter_resume_is_rejected() {
        let suspender = RuntimeSuspender::new();
        let handle = suspender.suspend_interpreter(
            10,
            vec![],
            OperandStack::new(1024),
            ControlStack::new(),
            1,
            0,
        );

        let mut interp = Interpreter::new().with_suspender(RuntimeSuspender::new());
        interp.interpreter_id = 1;
        interp.active_suspension_id = Some(handle.instance_id());
        interp.suspension_epoch = 0;
        interp.needs_resume = true;
        let other_thread_id = std::thread::spawn(|| std::thread::current().id())
            .join()
            .unwrap();
        interp.execution_thread = Some(other_thread_id);

        let result = interp.try_resume(&handle);
        assert!(matches!(
            result,
            Err(WasmError::Runtime(message)) if message.contains("cross-thread interpreter resume")
        ));
    }

    #[test]
    fn test_memory_visible_after_suspend_and_resume() {
        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.memories.push(MemoryType::new(Limits::Min(1)));
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x41, 0x2A, 0x36, 0x02, 0x00, 0x41, 0x00, 0x28, 0x02, 0x00, 0x0B,
            ],
        });

        let module = Arc::new(module);
        let instance = Arc::new(Mutex::new(Instance::new(module.clone()).unwrap()));
        let mut interp = Interpreter::with_instance(instance.clone())
            .with_suspender(RuntimeSuspender::new())
            .with_safepoints(SafepointConfig::new(true).with_interval(4));

        let mut result = interp.execute_function(&module, 0, &[]);
        let final_result = loop {
            match result {
                Ok(values) => break values,
                Err(WasmError::Suspended(SuspensionKind::Safepoint)) => {
                    let handle = interp
                        .take_suspended_handle()
                        .expect("should have suspended handle");
                    interp.try_resume(&handle).unwrap();
                    result = interp.continue_execution(&module);
                }
                Err(error) => panic!("unexpected error: {error:?}"),
            }
        };

        assert_eq!(final_result, vec![WasmValue::I32(42)]);

        let memory = instance.lock().unwrap().memory(0).cloned().unwrap();
        let value = memory.lock().unwrap().read_u32(0).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_imported_hostcall_pending_suspends_execution() {
        struct PendingHost {
            calls: AtomicUsize,
        }

        impl HostFunc for PendingHost {
            fn call(
                &self,
                _store: &mut crate::runtime::Store,
                _args: &[WasmValue],
            ) -> Result<Vec<WasmValue>> {
                panic!("pending hostcall should not fall back to synchronous completion")
            }

            fn call_with_suspension(
                &self,
                _store: &mut crate::runtime::Store,
                _args: &[WasmValue],
            ) -> Result<HostCallOutcome> {
                self.calls.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(HostCallOutcome::Pending {
                    pending_work: vec![9, 9, 9],
                })
            }

            fn function_type(&self) -> Option<&FunctionType> {
                static FUNC_TYPE: std::sync::OnceLock<FunctionType> = std::sync::OnceLock::new();
                Some(
                    FUNC_TYPE.get_or_init(|| {
                        FunctionType::new(vec![], vec![ValType::Num(NumType::I32)])
                    }),
                )
            }
        }

        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.imports.push(Import {
            module: "env".to_string(),
            name: "host".to_string(),
            kind: ImportKind::Func(0),
        });
        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x10, 0x00, 0x0B],
        });

        let module = Arc::new(module);
        let host = std::sync::Arc::new(PendingHost {
            calls: AtomicUsize::new(0),
        });
        let instance = Instance::with_imports(
            module.clone(),
            &[("env", "host", Extern::HostFunc(host.clone()))],
        )
        .unwrap();
        let instance = Arc::new(Mutex::new(instance));

        let mut interp =
            Interpreter::with_instance(instance).with_suspender(RuntimeSuspender::new());
        let first = interp.execute_function(&module, 1, &[]);
        assert!(matches!(
            first,
            Err(WasmError::Suspended(SuspensionKind::HostcallPending))
        ));

        let handle = interp
            .take_suspended_handle()
            .expect("pending hostcall should store suspended handle");
        assert_eq!(handle.pending_work(), Some(vec![9, 9, 9]));

        interp
            .resume_hostcall(&handle, &[WasmValue::I32(7)])
            .unwrap();
        let result = interp.continue_execution(&module).unwrap();
        assert_eq!(result, vec![WasmValue::I32(7)]);
        assert_eq!(host.calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[test]
    fn test_atomic_load_store() {
        use crate::memory::Memory;
        use std::sync::Arc;

        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x41, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x36, 0x02, 0x00, 0x41, 0x00, 0x28,
                0x02, 0x00, 0x0B,
            ],
        });

        let module = Arc::new(module);
        let mut instance = Instance::new(module.clone()).unwrap();
        let memory = Arc::new(Mutex::new(Memory::new(MemoryType::new(Limits::Min(1)))));
        instance.memories.push(memory.clone());
        let instance = Arc::new(Mutex::new(instance));

        let mut interp = Interpreter::with_instance(instance);
        let result = interp.execute_function(&module, 0, &[WasmValue::I32(42)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_atomic_rmw_add() {
        use std::sync::Arc;

        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0x41, 0x05, 0xFE, 0x12, 0x02, 0x00, 0x0B],
        });

        let module = Arc::new(module);
        let instance = Instance::new(module.clone()).unwrap();
        instance.memories[0]
            .lock()
            .unwrap()
            .write_i32(0, 10)
            .unwrap();
        let instance = Arc::new(Mutex::new(instance));

        let mut interp = Interpreter::with_instance(instance);
        let result = interp.execute_function(&module, 0, &[]);
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0], WasmValue::I32(10));
    }

    #[test]
    fn test_atomic_load() {
        use std::sync::Arc;

        let mut module = Module::new();
        module.types.push(FunctionType::new(
            vec![ValType::Num(NumType::I32)],
            vec![ValType::Num(NumType::I32)],
        ));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![0x41, 0x00, 0xFE, 0x00, 0x02, 0x00, 0x0B],
        });

        let module = Arc::new(module);
        let instance = Instance::new(module.clone()).unwrap();
        instance.memories[0]
            .lock()
            .unwrap()
            .write_i32(0, 0x12345678)
            .unwrap();
        let instance = Arc::new(Mutex::new(instance));

        let mut interp = Interpreter::with_instance(instance);
        let result = interp.execute_function(&module, 0, &[WasmValue::I32(0)]);
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0], WasmValue::I32(0x12345678));
    }

    #[test]
    fn test_atomic_rmw_cmpxchg_success() {
        use std::sync::Arc;

        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x41, 0x0A, 0x41, 0x14, 0xFE, 0x1E, 0x02, 0x00, 0x0B,
            ],
        });

        let module = Arc::new(module);
        let instance = Instance::new(module.clone()).unwrap();
        instance.memories[0]
            .lock()
            .unwrap()
            .write_i32(0, 10)
            .unwrap();
        let instance = Arc::new(Mutex::new(instance));

        let mut interp = Interpreter::with_instance(instance);
        let result = interp.execute_function(&module, 0, &[]);
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0], WasmValue::I32(10));
    }

    #[test]
    fn test_atomic_rmw_cmpxchg_fail() {
        use std::sync::Arc;

        let mut module = Module::new();
        module
            .types
            .push(FunctionType::new(vec![], vec![ValType::Num(NumType::I32)]));
        module.memories.push(MemoryType::new(Limits::Min(1)));

        module.funcs.push(Func {
            type_idx: 0,
            locals: vec![],
            body: vec![
                0x41, 0x00, 0x41, 0x0B, 0x41, 0x14, 0xFE, 0x1E, 0x02, 0x00, 0x0B,
            ],
        });

        let module = Arc::new(module);
        let instance = Instance::new(module.clone()).unwrap();
        instance.memories[0]
            .lock()
            .unwrap()
            .write_i32(0, 10)
            .unwrap();
        let instance = Arc::new(Mutex::new(instance));

        let mut interp = Interpreter::with_instance(instance);
        let result = interp.execute_function(&module, 0, &[]);
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert_eq!(returned.len(), 1);
        assert_eq!(returned[0], WasmValue::I32(10));
    }
}
