//! Snapshot and restore support for WebAssembly instances.
//!
//! This module provides APIs for capturing and restoring WebAssembly instance state
//! at safepoints, enabling migration and serialization of guest execution state.

use std::sync::Arc;

use crate::runtime::WasmValue;
use crate::runtime::instance::Instance;
use crate::runtime::module::Module;
use crate::runtime::suspend::SuspendedHandle;
use crate::runtime::types::{GlobalType, MemoryType, TableType};

/// Snapshot format version used by serialised payloads.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
/// Complete snapshot payload for an instance.
pub struct SnapshotPayload {
    /// Snapshot format version.
    pub version: u32,
    /// Hash of the source module used for compatibility checks.
    pub module_hash: u64,
    /// Snapshots of the instance memories.
    pub memory_snapshots: Vec<MemorySnapshot>,
    /// Snapshots of the instance globals.
    pub global_snapshots: Vec<GlobalSnapshot>,
    /// Snapshots of the instance tables.
    pub table_snapshots: Vec<TableSnapshot>,
    /// Captured execution state, if the instance was suspended.
    pub execution_state: Option<ExecutionStateSnapshot>,
    /// Snapshot metadata used for bookkeeping.
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone)]
/// Metadata associated with a snapshot payload.
pub struct SnapshotMetadata {
    /// Timestamp recorded when the snapshot was captured.
    pub timestamp: u64,
    /// Source instance identifier, if one was available.
    pub source_instance_id: Option<u64>,
}

#[derive(Debug, Clone)]
/// Snapshot of a single linear memory.
pub struct MemorySnapshot {
    /// The zero-based index.
    pub index: u32,
    /// Declared type of the captured memory.
    pub memory_type: MemoryType,
    /// Raw linear-memory bytes.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
/// Snapshot of a single global.
pub struct GlobalSnapshot {
    /// The zero-based index.
    pub index: u32,
    /// Declared type of the captured global.
    pub global_type: GlobalType,
    /// Captured value of the global.
    pub value: WasmValue,
}

#[derive(Debug, Clone)]
/// Snapshot of a single table.
pub struct TableSnapshot {
    /// The zero-based index.
    pub index: u32,
    /// Declared type of the captured table.
    pub table_type: TableType,
    /// Captured table elements.
    pub elements: Vec<WasmValue>,
}

#[derive(Debug, Clone)]
/// Captured execution state for a suspended instance.
pub struct ExecutionStateSnapshot {
    /// Interpreter-specific execution state.
    pub interpreter_state: InterpreterStateSnapshot,
}

#[derive(Debug, Clone)]
/// Captured interpreter state needed to resume execution.
pub struct InterpreterStateSnapshot {
    /// Program-counter offset within the current function body.
    pub pc: usize,
    /// Captured locals for the current frame.
    pub locals: Vec<WasmValue>,
    /// Captured operand-stack contents.
    pub operand_stack: Vec<WasmValue>,
    /// Maximum capacity of the operand stack.
    pub operand_stack_max_size: usize,
    /// Serialised control-stack state.
    pub control_stack: Vec<u8>,
    /// Identifier of the interpreter that produced the snapshot.
    pub interpreter_id: u64,
    /// Suspension epoch used to validate resumption.
    pub suspension_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors that can occur while capturing or restoring snapshots.
pub enum SnapshotError {
    /// The target instance is not currently suspended.
    InstanceNotSuspended(String),
    /// The snapshot contains a resource that is not currently supported.
    UnsupportedResource(String),
    /// The snapshot does not match the requested restore target.
    IncompatibleTarget(String),
    /// The snapshot payload is malformed or inconsistent.
    InvalidSnapshot(String),
    /// A snapshot operation is already in progress.
    SnapshotInProgress,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InstanceNotSuspended(msg) => {
                write!(f, "Instance not suspended: {}", msg)
            }
            Self::UnsupportedResource(msg) => {
                write!(f, "Unsupported resource: {}", msg)
            }
            Self::IncompatibleTarget(msg) => {
                write!(f, "Incompatible target: {}", msg)
            }
            Self::InvalidSnapshot(msg) => {
                write!(f, "Invalid snapshot: {}", msg)
            }
            Self::SnapshotInProgress => write!(f, "Snapshot already in progress"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Type alias for `Result`.
pub type Result<T> = std::result::Result<T, SnapshotError>;

fn compute_module_hash(module: &Module) -> u64 {
    use sha2::{Digest, Sha256};
    use std::io::Write;

    let mut hasher = Sha256::new();

    hasher.write_all(&module.types.len().to_le_bytes()).unwrap();
    hasher.write_all(&module.funcs.len().to_le_bytes()).unwrap();
    hasher
        .write_all(&module.tables.len().to_le_bytes())
        .unwrap();
    hasher
        .write_all(&module.memories.len().to_le_bytes())
        .unwrap();
    hasher
        .write_all(&module.globals.len().to_le_bytes())
        .unwrap();
    hasher
        .write_all(&module.exports.len().to_le_bytes())
        .unwrap();
    hasher
        .write_all(&module.imports.len().to_le_bytes())
        .unwrap();
    hasher.write_all(&module.data.len().to_le_bytes()).unwrap();
    hasher.write_all(&module.elems.len().to_le_bytes()).unwrap();

    for func in &module.funcs {
        hasher.write_all(&func.type_idx.to_le_bytes()).unwrap();
    }

    for mem in &module.memories {
        hasher.write_all(&mem.limits.min().to_le_bytes()).unwrap();
        if let Some(max) = mem.limits.max() {
            hasher.write_all(&max.to_le_bytes()).unwrap();
        }
    }

    for global_type in &module.globals {
        hasher
            .write_all(&global_type.content_type.hash_bytes())
            .unwrap();
        hasher.write_all(&[global_type.mutable as u8]).unwrap();
    }

    for table in &module.tables {
        hasher.write_all(&[table.elem_type as u8]).unwrap();
        hasher
            .write_all(&(table.limits.min() as u64).to_le_bytes())
            .unwrap();
        if let Some(max) = table.limits.max() {
            hasher.write_all(&(max as u64).to_le_bytes()).unwrap();
        }
    }

    for export in &module.exports {
        hasher.write_all(export.name.as_bytes()).unwrap();
    }

    for import in &module.imports {
        hasher.write_all(import.module.as_bytes()).unwrap();
        hasher.write_all(import.name.as_bytes()).unwrap();
    }

    for data in &module.data {
        let kind_byte: u8 = match data.kind {
            crate::runtime::module::DataKind::Active { .. } => 0,
            crate::runtime::module::DataKind::Passive => 1,
        };
        hasher.write_all(&[kind_byte]).unwrap();
        hasher
            .write_all(&(data.init.len() as u64).to_le_bytes())
            .unwrap();
        hasher.write_all(&data.init).unwrap();
    }

    for elem in &module.elems {
        let kind_byte: u8 = match elem.kind {
            crate::runtime::module::ElemKind::Active { .. } => 0,
            crate::runtime::module::ElemKind::Passive => 1,
            crate::runtime::module::ElemKind::Declarative => 2,
        };
        hasher.write_all(&[kind_byte]).unwrap();
        hasher
            .write_all(&(elem.init.len() as u64).to_le_bytes())
            .unwrap();
        for elem_init in &elem.init {
            hasher.write_all(elem_init).unwrap();
        }
    }

    let result = hasher.finalize();
    u64::from_le_bytes([
        result[0], result[1], result[2], result[3], result[4], result[5], result[6], result[7],
    ])
}

/// Captures snapshot.
pub fn capture_snapshot(
    instance: &Instance,
    suspended_handle: Option<&SuspendedHandle>,
) -> Result<SnapshotPayload> {
    let module = instance.module();

    let execution_state = if let Some(handle) = suspended_handle {
        let state = handle.suspension_state();
        match state {
            crate::runtime::suspend::SuspensionState::Interpreter(interpreter_state) => {
                Some(ExecutionStateSnapshot {
                    interpreter_state: InterpreterStateSnapshot {
                        pc: interpreter_state.pc,
                        locals: interpreter_state.locals,
                        operand_stack: interpreter_state.operand_stack.to_vec(),
                        operand_stack_max_size: interpreter_state.operand_stack.max_size(),
                        control_stack: interpreter_state.control_stack.to_bytes(),
                        interpreter_id: interpreter_state.interpreter_id,
                        suspension_epoch: interpreter_state.suspension_epoch,
                    },
                })
            }
            crate::runtime::suspend::SuspensionState::Jit(_) => {
                return Err(SnapshotError::UnsupportedResource(
                    "JIT execution state cannot be captured in snapshots".to_string(),
                ));
            }
            crate::runtime::suspend::SuspensionState::HostcallPending { .. } => {
                return Err(SnapshotError::UnsupportedResource(
                    "Hostcall pending state cannot be captured in snapshots".to_string(),
                ));
            }
            crate::runtime::suspend::SuspensionState::None => None,
        }
    } else {
        None
    };

    let mut memory_snapshots = Vec::new();
    for (idx, memory) in instance.memories.iter().enumerate() {
        let mem = memory
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock memory".to_string()))?;
        let memory_type = mem.type_().clone();
        let data = mem.data().to_vec();
        memory_snapshots.push(MemorySnapshot {
            index: idx as u32,
            memory_type,
            data,
        });
    }

    let mut global_snapshots = Vec::new();
    for (idx, global) in instance.globals.iter().enumerate() {
        let g = global
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock global".to_string()))?;
        global_snapshots.push(GlobalSnapshot {
            index: idx as u32,
            global_type: g.type_.clone(),
            value: g.value,
        });
    }

    let mut table_snapshots = Vec::new();
    for (idx, table) in instance.tables.iter().enumerate() {
        let t = table
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock table".to_string()))?;
        table_snapshots.push(TableSnapshot {
            index: idx as u32,
            table_type: t.type_.clone(),
            elements: t.data.clone(),
        });
    }

    let metadata = SnapshotMetadata {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        source_instance_id: suspended_handle.map(|h| h.instance_id()),
    };

    Ok(SnapshotPayload {
        version: SNAPSHOT_FORMAT_VERSION,
        module_hash: compute_module_hash(module),
        memory_snapshots,
        global_snapshots,
        table_snapshots,
        execution_state,
        metadata,
    })
}

/// Validates snapshot compatibility.
pub fn validate_snapshot_compatibility(
    snapshot: &SnapshotPayload,
    target_module: &Module,
) -> Result<()> {
    if snapshot.version != SNAPSHOT_FORMAT_VERSION {
        return Err(SnapshotError::IncompatibleTarget(format!(
            "snapshot format version {} is not compatible with runtime version {}",
            snapshot.version, SNAPSHOT_FORMAT_VERSION
        )));
    }

    let target_hash = compute_module_hash(target_module);
    if snapshot.module_hash != target_hash {
        return Err(SnapshotError::IncompatibleTarget(
            "module hash mismatch: snapshot was created from a different module".to_string(),
        ));
    }

    Ok(())
}

/// Restores snapshot.
pub fn restore_snapshot(
    snapshot: &SnapshotPayload,
    target_module: Arc<Module>,
    target_instance: &mut Instance,
) -> Result<Option<crate::runtime::suspend::SuspendedHandle>> {
    validate_snapshot_compatibility(snapshot, &target_module)?;

    for memory_snapshot in &snapshot.memory_snapshots {
        let memory = target_instance
            .memory(memory_snapshot.index)
            .ok_or_else(|| {
                SnapshotError::InvalidSnapshot(format!(
                    "memory {} not found in target instance",
                    memory_snapshot.index
                ))
            })?;

        let mut mem = memory
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock memory".to_string()))?;

        let target_bytes = mem.size() as usize * 65536;
        if target_bytes < memory_snapshot.data.len() {
            return Err(SnapshotError::InvalidSnapshot(
                "target memory is too small for snapshot data".to_string(),
            ));
        }

        if mem.type_() != &memory_snapshot.memory_type {
            return Err(SnapshotError::IncompatibleTarget(
                "memory type mismatch".to_string(),
            ));
        }

        mem.data_mut().copy_from_slice(&memory_snapshot.data);
    }

    for global_snapshot in &snapshot.global_snapshots {
        let global = target_instance
            .global(global_snapshot.index)
            .ok_or_else(|| {
                SnapshotError::InvalidSnapshot(format!(
                    "global {} not found in target instance",
                    global_snapshot.index
                ))
            })?;

        let mut g = global
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock global".to_string()))?;

        if g.type_ != global_snapshot.global_type {
            return Err(SnapshotError::IncompatibleTarget(format!(
                "global {} type mismatch",
                global_snapshot.index
            )));
        }

        g.value = global_snapshot.value;
    }

    for table_snapshot in &snapshot.table_snapshots {
        let table = target_instance.table(table_snapshot.index).ok_or_else(|| {
            SnapshotError::InvalidSnapshot(format!(
                "table {} not found in target instance",
                table_snapshot.index
            ))
        })?;

        let mut t = table
            .lock()
            .map_err(|_| SnapshotError::InvalidSnapshot("failed to lock table".to_string()))?;

        if t.type_ != table_snapshot.table_type {
            return Err(SnapshotError::IncompatibleTarget(format!(
                "table {} type mismatch",
                table_snapshot.index
            )));
        }

        if t.size() < table_snapshot.elements.len() as u32 {
            return Err(SnapshotError::InvalidSnapshot(
                "target table is too small for snapshot data".to_string(),
            ));
        }

        t.data = table_snapshot.elements.clone();
    }

    let suspended_handle = if let Some(ref exec_state) = snapshot.execution_state {
        let control_stack = crate::interpreter::stack::ControlStack::from_bytes(
            &exec_state.interpreter_state.control_stack,
        );
        let interpreter_state = crate::runtime::suspend::InterpreterState::restore_from_snapshot(
            exec_state.interpreter_state.pc,
            exec_state.interpreter_state.locals.clone(),
            crate::interpreter::stack::OperandStack::from_vec(
                exec_state.interpreter_state.operand_stack.clone(),
                exec_state.interpreter_state.operand_stack_max_size,
            ),
            control_stack,
            exec_state.interpreter_state.interpreter_id,
            exec_state.interpreter_state.suspension_epoch,
        );
        let suspended_instance = crate::runtime::suspend::SuspendedInstance::new_interpreter(
            interpreter_state,
            snapshot.metadata.source_instance_id.unwrap_or(0),
        );
        Some(crate::runtime::suspend::SuspendedHandle::new(
            suspended_instance,
        ))
    } else {
        None
    };

    if snapshot.execution_state.is_some() && suspended_handle.is_none() {
        return Err(SnapshotError::InvalidSnapshot(
            "failed to restore execution state".to_string(),
        ));
    }

    Ok(suspended_handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Limits, Module};

    #[test]
    fn test_snapshot_payload_creation() {
        let module = Arc::new(Module::new());
        let instance = Instance::new(module).unwrap();

        let snapshot = capture_snapshot(&instance, None).unwrap();

        assert_eq!(snapshot.version, SNAPSHOT_FORMAT_VERSION);
        assert_eq!(snapshot.memory_snapshots.len(), 0);
        assert_eq!(snapshot.global_snapshots.len(), 0);
        assert_eq!(snapshot.table_snapshots.len(), 0);
        assert!(snapshot.execution_state.is_none());
    }

    #[test]
    fn test_snapshot_memory_contents() {
        let mut module = Module::new();
        module.memories.push(MemoryType::new(Limits::Min(1)));

        let mut instance = Instance::new(Arc::new(module)).unwrap();

        instance
            .memory_mut(0)
            .unwrap()
            .lock()
            .unwrap()
            .write(0, &[1, 2, 3, 4])
            .unwrap();

        let snapshot = capture_snapshot(&instance, None).unwrap();

        assert_eq!(snapshot.memory_snapshots.len(), 1);
        assert_eq!(snapshot.memory_snapshots[0].data[..4], [1, 2, 3, 4]);
    }

    #[test]
    fn test_snapshot_global_values() {
        let mut module = Module::new();
        module.globals.push(crate::runtime::types::GlobalType::new(
            crate::runtime::types::ValType::Num(crate::runtime::types::NumType::I32),
            false,
        ));
        module.global_inits.push(vec![0x41, 0x00, 0x0B]);

        let instance = Instance::new(Arc::new(module)).unwrap();

        let snapshot = capture_snapshot(&instance, None).unwrap();

        assert!(!snapshot.global_snapshots.is_empty());
    }

    #[test]
    fn test_snapshot_incompatible_version() {
        let module = Arc::new(Module::new());
        let instance = Instance::new(module.clone()).unwrap();

        let mut snapshot = capture_snapshot(&instance, None).unwrap();
        snapshot.version = 9999;

        let result = validate_snapshot_compatibility(&snapshot, &module);
        assert!(matches!(result, Err(SnapshotError::IncompatibleTarget(_))));
    }

    #[test]
    fn test_restore_snapshot_round_trip() {
        let mut module = Module::new();
        module.globals.push(crate::runtime::types::GlobalType::new(
            crate::runtime::types::ValType::Num(crate::runtime::types::NumType::I32),
            true,
        ));
        module.global_inits.push(vec![0x41, 0x00, 0x0B]);

        let mut instance = Instance::new(Arc::new(module.clone())).unwrap();

        instance
            .global_mut(0)
            .unwrap()
            .lock()
            .unwrap()
            .set(WasmValue::I32(42))
            .unwrap();

        let snapshot = capture_snapshot(&instance, None).unwrap();

        let mut restored_instance = Instance::new(Arc::new(module.clone())).unwrap();
        restore_snapshot(&snapshot, Arc::new(module.clone()), &mut restored_instance).unwrap();

        let global_value = restored_instance.global(0).unwrap().lock().unwrap().get();
        assert_eq!(global_value, WasmValue::I32(42));
    }

    #[test]
    fn test_snapshot_module_hash_mismatch() {
        let mut module = Module::new();
        module.funcs.push(crate::runtime::module::Func {
            type_idx: 0,
            locals: vec![],
            body: vec![],
        });
        let module = Arc::new(module);
        let instance = Instance::new(module.clone()).unwrap();

        let snapshot = capture_snapshot(&instance, None).unwrap();

        let different_module = Arc::new(Module::new());
        let result = validate_snapshot_compatibility(&snapshot, &different_module);
        assert!(matches!(result, Err(SnapshotError::IncompatibleTarget(_))));
    }

    #[test]
    fn test_snapshot_execution_state_capture() {
        use crate::runtime::suspend::{InterpreterState, SuspendedHandle, SuspendedInstance};

        let module = Arc::new(Module::new());
        let instance = Instance::new(module).unwrap();

        let suspended_instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 100,
                locals: vec![WasmValue::I32(123)],
                operand_stack: {
                    let mut stack = crate::interpreter::stack::OperandStack::new(1024);
                    let _ = stack.push(WasmValue::I32(456));
                    stack
                },
                control_stack: crate::interpreter::stack::ControlStack::new(),
                interpreter_id: 1,
                suspension_epoch: 42,
            },
            1,
        );
        let handle = SuspendedHandle::new(suspended_instance);

        let snapshot = capture_snapshot(&instance, Some(&handle)).unwrap();

        assert!(snapshot.execution_state.is_some());
        let exec_state = snapshot.execution_state.as_ref().unwrap();
        assert_eq!(exec_state.interpreter_state.pc, 100);
        assert_eq!(exec_state.interpreter_state.locals.len(), 1);
    }

    #[test]
    fn test_restore_execution_state_success() {
        use crate::runtime::suspend::{InterpreterState, SuspendedHandle, SuspendedInstance};

        let module = Arc::new(Module::new());
        let instance = Instance::new(module.clone()).unwrap();

        let mut control_stack = crate::interpreter::stack::ControlStack::new();
        control_stack.push(crate::interpreter::stack::ControlFrame {
            kind: crate::interpreter::stack::FrameKind::Function,
            position: 42,
            code: vec![],
            arity: 1,
            label_arity: 0,
            local_count: 2,
            height: 10,
            locals: vec![WasmValue::I32(1), WasmValue::I32(2)],
        });

        let suspended_instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 100,
                locals: vec![WasmValue::I32(42)],
                operand_stack: crate::interpreter::stack::OperandStack::new(1024),
                control_stack,
                interpreter_id: 1,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(suspended_instance);

        let snapshot = capture_snapshot(&instance, Some(&handle)).unwrap();
        assert!(snapshot.execution_state.is_some());

        let mut target_instance = Instance::new(module.clone()).unwrap();
        let result = restore_snapshot(&snapshot, module, &mut target_instance);
        let restored_handle = result.unwrap();
        assert!(restored_handle.is_some());

        let restored_handle = restored_handle.unwrap();
        let state = restored_handle.suspension_state();
        if let crate::runtime::suspend::SuspensionState::Interpreter(interp_state) = state {
            assert_eq!(interp_state.pc, 100);
            assert_eq!(interp_state.locals.len(), 1);
            assert_eq!(interp_state.control_stack.frames().len(), 1);
            let frame = &interp_state.control_stack.frames()[0];
            assert_eq!(frame.position, 42);
            assert_eq!(frame.arity, 1);
        } else {
            panic!("expected interpreter state");
        }
    }

    #[test]
    fn test_restore_execution_state_fails_with_incompatible_module() {
        use crate::runtime::suspend::{InterpreterState, SuspendedHandle, SuspendedInstance};

        let module = Arc::new(Module::new());
        let instance = Instance::new(module.clone()).unwrap();

        let suspended_instance = SuspendedInstance::new_interpreter(
            InterpreterState {
                pc: 0,
                locals: vec![],
                operand_stack: crate::interpreter::stack::OperandStack::new(1024),
                control_stack: crate::interpreter::stack::ControlStack::new(),
                interpreter_id: 1,
                suspension_epoch: 0,
            },
            1,
        );
        let handle = SuspendedHandle::new(suspended_instance);

        let snapshot = capture_snapshot(&instance, Some(&handle)).unwrap();

        let mut different_module = Module::new();
        different_module.funcs.push(crate::runtime::module::Func {
            type_idx: 0,
            locals: vec![],
            body: vec![],
        });
        let different_module = Arc::new(different_module);
        let mut target_instance = Instance::new(different_module.clone()).unwrap();
        let result = restore_snapshot(&snapshot, different_module, &mut target_instance);
        assert!(matches!(result, Err(SnapshotError::IncompatibleTarget(_))));
    }
}
