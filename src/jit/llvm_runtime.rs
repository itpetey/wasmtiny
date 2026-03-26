use crate::aot_runtime::runtime::AotModule;
use crate::runtime::{
    NumType, RuntimeSuspender, SharedMemoryMappingId, SuspendedHandle, TrapCode, ValType,
    WasmError, WasmValue,
};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ptr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

thread_local! {
    static LLVM_RUNTIME_CTX: RefCell<LlvmRuntimeContext> =
        const { RefCell::new(LlvmRuntimeContext::new()) };
}

struct LlvmRuntimeContext {
    memory_ptr: *mut u8,
    memory_len: usize,
    current_module: *mut AotModule,
    current_context_id: Option<u64>,
    current_module_fingerprint: Option<u64>,
    current_owner_jit_id: Option<u64>,
    executing: bool,
    trap: Option<TrapCode>,
    safepoints_enabled: bool,
    suspend_requested: bool,
    jit_id: u64,
    execution_epoch: u64,
    suspended_handle: Option<SuspendedHandle>,
    runtime_error: Option<String>,
    execution_flag: Option<Arc<AtomicBool>>,
    execution_pinned: bool,
}

impl LlvmRuntimeContext {
    const fn new() -> Self {
        Self {
            memory_ptr: ptr::null_mut(),
            memory_len: 0,
            current_module: ptr::null_mut(),
            current_context_id: None,
            current_module_fingerprint: None,
            current_owner_jit_id: None,
            executing: false,
            trap: None,
            safepoints_enabled: false,
            suspend_requested: false,
            jit_id: 0,
            execution_epoch: 0,
            suspended_handle: None,
            runtime_error: None,
            execution_flag: None,
            execution_pinned: false,
        }
    }
}

fn context_id_for_module(module: *mut AotModule) -> Option<u64> {
    (!module.is_null()).then(|| unsafe { (&*module).runtime_id() })
}

fn fingerprint_for_module(module: *mut AotModule) -> Option<u64> {
    if module.is_null() {
        return None;
    }

    let mut hasher = DefaultHasher::new();
    format!("{:?}", unsafe { (&*module).module() }).hash(&mut hasher);
    Some(hasher.finish())
}

/// Sets the execution context for the current thread.
///
/// This must be called before invoking compiled WASM code so runtime helpers can
/// reach the active module, linear memory, and trap state.
///
/// # Safety
///
/// `module` must either be null or point to a live, stable `AotModule` for the
/// full duration of any compiled code that may access the current thread-local
/// execution context.
pub unsafe fn set_execution_context(
    module: *mut AotModule,
    memory_ptr: *mut u8,
    memory_len: usize,
    owner_jit_id: Option<u64>,
) -> Result<()> {
    let refresh_pinned_context = LLVM_RUNTIME_CTX.with(|ctx| {
        let ctx = ctx.borrow();
        let same_owner = ctx.current_owner_jit_id == owner_jit_id;
        let same_module = ctx.current_module == module;

        if ctx.execution_pinned {
            if same_owner && same_module {
                return Ok(true);
            }
            return Err(WasmError::Runtime(
                "cannot replace execution context while JIT execution is suspended".to_string(),
            ));
        }
        if ctx.executing {
            return Err(WasmError::Runtime(
                "cannot replace execution context while JIT execution is active".to_string(),
            ));
        }
        if ctx.current_owner_jit_id.is_some() && ctx.current_owner_jit_id != owner_jit_id {
            return Err(WasmError::Runtime(
                "cannot replace execution context owned by a different JIT".to_string(),
            ));
        }
        Ok(false)
    })?;

    if refresh_pinned_context {
        LLVM_RUNTIME_CTX.with(|ctx| {
            let mut ctx = ctx.borrow_mut();
            ctx.current_context_id = context_id_for_module(module);
            ctx.current_module_fingerprint = fingerprint_for_module(module);
            ctx.memory_ptr = memory_ptr;
            ctx.memory_len = memory_len;
            ctx.trap = None;
            ctx.suspended_handle = None;
            ctx.runtime_error = None;
        });
        return Ok(());
    }

    clear_execution_context();
    let execution_flag = if module.is_null() {
        None
    } else {
        Some(unsafe { (&*module).try_begin_jit_execution()? })
    };

    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.current_module = module;
        ctx.current_context_id = context_id_for_module(module);
        ctx.current_module_fingerprint = fingerprint_for_module(module);
        ctx.current_owner_jit_id = owner_jit_id;
        ctx.executing = false;
        ctx.memory_ptr = memory_ptr;
        ctx.memory_len = memory_len;
        ctx.trap = None;
        ctx.suspended_handle = None;
        ctx.runtime_error = None;
        ctx.execution_flag = execution_flag;
        ctx.execution_pinned = false;
    });

    Ok(())
}

/// Returns whether this value has execution context.
pub fn has_execution_context() -> bool {
    LLVM_RUNTIME_CTX.with(|ctx| !ctx.borrow().current_module.is_null())
}

/// Returns the active execution context identifier, if any.
pub fn current_execution_context_id() -> Option<u64> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow().current_context_id)
}

/// Returns the active module fingerprint, if any.
pub fn current_execution_module_fingerprint() -> Option<u64> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow().current_module_fingerprint)
}

/// Returns the owning JIT identifier for the active execution context.
pub fn current_execution_owner_jit_id() -> Option<u64> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow().current_owner_jit_id)
}

/// Sets only the memory context for the current thread.
///
/// This is kept for compatibility with existing tests and callers that do not
/// need module-backed import dispatch.
pub fn set_memory_context(ptr: *mut u8, len: usize) {
    unsafe { set_execution_context(ptr::null_mut(), ptr, len, None) }
        .expect("null module context cannot fail");
}

/// Clears execution context.
pub fn clear_execution_context() {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if ctx.execution_pinned {
            return;
        }
        if let Some(flag) = ctx.execution_flag.take() {
            flag.store(false, Ordering::SeqCst);
        }
        ctx.current_module = ptr::null_mut();
        ctx.current_context_id = None;
        ctx.current_module_fingerprint = None;
        ctx.current_owner_jit_id = None;
        ctx.executing = false;
        ctx.memory_ptr = ptr::null_mut();
        ctx.memory_len = 0;
        ctx.suspended_handle = None;
        ctx.execution_pinned = false;
    });
}

/// Clears execution context for owner.
pub fn clear_execution_context_for_owner(owner_jit_id: u64, force: bool) -> bool {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if ctx.current_owner_jit_id != Some(owner_jit_id) {
            return false;
        }
        if ctx.execution_pinned && !force {
            return false;
        }
        if let Some(flag) = ctx.execution_flag.take() {
            flag.store(false, Ordering::SeqCst);
        }
        ctx.current_module = ptr::null_mut();
        ctx.current_context_id = None;
        ctx.current_module_fingerprint = None;
        ctx.current_owner_jit_id = None;
        ctx.executing = false;
        ctx.memory_ptr = ptr::null_mut();
        ctx.memory_len = 0;
        ctx.suspended_handle = None;
        ctx.execution_pinned = false;
        true
    })
}

/// Sets execution context pinned for owner.
pub fn set_execution_context_pinned_for_owner(owner_jit_id: u64, pinned: bool) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if ctx.current_owner_jit_id != Some(owner_jit_id) {
            return;
        }
        ctx.execution_pinned = pinned;
    })
}

/// Clears trap.
pub fn clear_trap() {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.trap = None;
        ctx.runtime_error = None;
    });
}

/// Configures safepoint handling for the current execution context.
pub fn configure_safepoints(enabled: bool, requested: bool, jit_id: u64, execution_epoch: u64) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.safepoints_enabled = enabled;
        ctx.suspend_requested = requested;
        ctx.jit_id = jit_id;
        ctx.execution_epoch = execution_epoch;
        ctx.executing = true;
        ctx.suspended_handle = None;
        ctx.runtime_error = None;
    });
}

/// Takes suspended handle.
pub fn take_suspended_handle() -> Option<SuspendedHandle> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow_mut().suspended_handle.take())
}

/// Takes runtime error.
pub fn take_runtime_error() -> Option<String> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow_mut().runtime_error.take())
}

/// Takes trap.
pub fn take_trap() -> Option<TrapCode> {
    LLVM_RUNTIME_CTX.with(|ctx| ctx.borrow_mut().trap.take())
}

fn set_trap(code: TrapCode) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if ctx.trap.is_none() {
            ctx.trap = Some(code);
        }
    });
}

fn trap_code_to_i32(code: TrapCode) -> i32 {
    match code {
        TrapCode::Unreachable => 1,
        TrapCode::MemoryOutOfBounds => 2,
        TrapCode::MemoryLimitExceeded => 3,
        TrapCode::TableOutOfBounds => 4,
        TrapCode::IndirectCallTypeMismatch => 5,
        TrapCode::StackOverflow => 6,
        TrapCode::ExecutionBudgetExceeded => 7,
        TrapCode::IntegerOverflow => 8,
        TrapCode::IntegerDivisionByZero => 9,
        TrapCode::InvalidConversionToInt => 10,
        TrapCode::CallIndirectNull => 11,
        TrapCode::NullReference => 12,
        TrapCode::HostTrap => 13,
    }
}

fn set_runtime_error(message: String) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        ctx.borrow_mut().runtime_error = Some(message);
    });
}

fn check_bounds(addr: u32, size: u32) -> Option<*mut u8> {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let ctx = ctx.borrow();
        if ctx.memory_ptr.is_null() {
            return None;
        }
        let end = (addr as usize).checked_add(size as usize)?;
        if end > ctx.memory_len {
            return None;
        }
        Some(unsafe { ctx.memory_ptr.add(addr as usize) })
    })
}

fn with_current_module_mut<F, T>(f: F) -> Option<T>
where
    F: FnOnce(&mut AotModule) -> T,
{
    LLVM_RUNTIME_CTX.with(|ctx| {
        let module_ptr = ctx.borrow().current_module;
        if module_ptr.is_null() {
            return None;
        }
        Some(unsafe { f(&mut *module_ptr) })
    })
}

fn refresh_memory_context_from_module() {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let current_module = ctx.borrow().current_module;
        if current_module.is_null() {
            return;
        }

        let (memory_ptr, memory_len) =
            unsafe { (&mut *current_module).memory_context() }.unwrap_or((ptr::null_mut(), 0));
        let mut ctx = ctx.borrow_mut();
        ctx.memory_ptr = memory_ptr;
        ctx.memory_len = memory_len;
    });
}

fn pack_wasm_value(value: &WasmValue) -> Option<u64> {
    match value {
        WasmValue::I32(v) => Some(*v as u32 as u64),
        WasmValue::I64(v) => Some(*v as u64),
        WasmValue::F32(v) => Some(v.to_bits() as u64),
        WasmValue::F64(v) => Some(v.to_bits()),
        _ => None,
    }
}

fn unpack_raw_value(raw: u64, value_type: ValType) -> Option<WasmValue> {
    match value_type {
        ValType::Num(NumType::I32) => Some(WasmValue::I32(raw as u32 as i32)),
        ValType::Num(NumType::I64) => Some(WasmValue::I64(raw as i64)),
        ValType::Num(NumType::F32) => Some(WasmValue::F32(f32::from_bits(raw as u32))),
        ValType::Num(NumType::F64) => Some(WasmValue::F64(f64::from_bits(raw))),
        _ => None,
    }
}

fn wasm_f32_min(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        return f32::from_bits(0x7fc0_0000);
    }
    if a == b {
        if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
            return -0.0;
        }
        return a;
    }
    if a < b { a } else { b }
}

fn wasm_f64_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::from_bits(0x7ff8_0000_0000_0000);
    }
    if a == b {
        if a == 0.0 && (a.is_sign_negative() || b.is_sign_negative()) {
            return -0.0;
        }
        return a;
    }
    if a < b { a } else { b }
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_has_trap() -> i32 {
    LLVM_RUNTIME_CTX.with(|ctx| {
        ctx.borrow()
            .trap
            .as_ref()
            .map(|code| trap_code_to_i32(code.clone()))
            .unwrap_or(0)
    })
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_trap_unreachable() {
    set_trap(TrapCode::Unreachable);
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_call_import(
    func_idx: u32,
    args_ptr: *const u64,
    arg_count: u32,
    results_ptr: *mut u64,
    result_count: u32,
) {
    let Some(result) = with_current_module_mut(|module| {
        let Some(func_type) = module.get_func_type(func_idx).cloned() else {
            return Err(WasmError::Trap(TrapCode::HostTrap));
        };

        if func_type.params.len() != arg_count as usize
            || func_type.results.len() != result_count as usize
        {
            return Err(WasmError::Trap(TrapCode::HostTrap));
        }

        let mut args = Vec::with_capacity(func_type.params.len());
        for (idx, value_type) in func_type.params.iter().enumerate() {
            let raw = unsafe { *args_ptr.add(idx) };
            let Some(value) = unpack_raw_value(raw, *value_type) else {
                return Err(WasmError::Trap(TrapCode::HostTrap));
            };
            args.push(value);
        }

        module.invoke_import_with_suspension(func_idx, &args)
    }) else {
        set_trap(TrapCode::HostTrap);
        return;
    };

    match result {
        Ok(crate::runtime::HostCallOutcome::Complete(results)) => {
            if results.len() != result_count as usize {
                set_trap(TrapCode::HostTrap);
                refresh_memory_context_from_module();
                return;
            }
            for (idx, value) in results.iter().enumerate() {
                let Some(raw) = pack_wasm_value(value) else {
                    set_trap(TrapCode::HostTrap);
                    refresh_memory_context_from_module();
                    return;
                };
                unsafe {
                    *results_ptr.add(idx) = raw;
                }
            }
        }
        Ok(crate::runtime::HostCallOutcome::Pending { .. }) => {
            LLVM_RUNTIME_CTX.with(|ctx| {
                let mut ctx = ctx.borrow_mut();
                ctx.runtime_error = Some(
                    "pending hostcall suspension is unsupported in JIT import path".to_string(),
                );
            });
            set_trap(TrapCode::HostTrap);
            refresh_memory_context_from_module();
            return;
        }
        Err(WasmError::Trap(code)) => set_trap(code),
        Err(_) => set_trap(TrapCode::HostTrap),
    }

    refresh_memory_context_from_module();
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_safepoint_entry(
    func_idx: u32,
    args_ptr: *const u64,
    arg_count: u32,
) -> i32 {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        if !ctx.safepoints_enabled || !ctx.suspend_requested {
            return 0;
        }

        let Some(module_ptr) = (!ctx.current_module.is_null()).then_some(ctx.current_module) else {
            ctx.trap = Some(TrapCode::HostTrap);
            return 1;
        };

        let module = unsafe { &mut *module_ptr };
        let Some(func_type) = module.get_func_type(func_idx).cloned() else {
            ctx.trap = Some(TrapCode::HostTrap);
            return 1;
        };

        if func_type.params.len() != arg_count as usize {
            ctx.trap = Some(TrapCode::HostTrap);
            return 1;
        }

        let mut args = Vec::with_capacity(func_type.params.len());
        for (idx, value_type) in func_type.params.iter().enumerate() {
            let raw = unsafe { *args_ptr.add(idx) };
            let Some(value) = unpack_raw_value(raw, *value_type) else {
                ctx.trap = Some(TrapCode::HostTrap);
                return 1;
            };
            args.push(value);
        }

        let handle = RuntimeSuspender::new().suspend_jit(
            func_idx,
            args,
            ctx.jit_id,
            ctx.execution_epoch,
            ctx.current_context_id.unwrap_or(0),
        );
        ctx.suspended_handle = Some(handle);
        ctx.suspend_requested = false;
        1
    })
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_meter_tick(units: u64) {
    let Some(result) = with_current_module_mut(|module| module.record_execution(units)) else {
        set_runtime_error("missing JIT execution context for metering".to_string());
        set_trap(TrapCode::HostTrap);
        return;
    };

    match result {
        Ok(()) => {}
        Err(WasmError::Trap(code)) => set_trap(code),
        Err(error) => {
            set_runtime_error(error.to_string());
            set_trap(TrapCode::HostTrap);
        }
    }
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_memory_size() -> i32 {
    let Some(result) = with_current_module_mut(|module| module.memory_size(0)) else {
        set_trap(TrapCode::HostTrap);
        return 0;
    };

    match result {
        Ok(size) => size,
        Err(error) => {
            set_runtime_error(error.to_string());
            set_trap(TrapCode::HostTrap);
            0
        }
    }
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_memory_grow(delta: i32) -> i32 {
    let Some(result) = with_current_module_mut(|module| module.memory_grow_wasm(0, delta)) else {
        set_trap(TrapCode::HostTrap);
        return -1;
    };

    let value = match result {
        Ok(old_size) => old_size,
        Err(WasmError::Trap(code)) => {
            set_trap(code);
            -1
        }
        Err(error) => {
            set_runtime_error(error.to_string());
            set_trap(TrapCode::HostTrap);
            -1
        }
    };

    refresh_memory_context_from_module();
    value
}

macro_rules! define_shared_load {
    ($name:ident, $ty:ty, $default:expr, $call:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(mapping_id: u64, offset: u32) -> $ty {
            let mapping_id = SharedMemoryMappingId::from_raw(mapping_id);
            let Some(result) = with_current_module_mut(|module| $call(module, mapping_id, offset))
            else {
                set_runtime_error("missing JIT execution context for shared memory".to_string());
                set_trap(TrapCode::HostTrap);
                return $default;
            };

            match result {
                Ok(value) => value,
                Err(WasmError::Trap(code)) => {
                    set_trap(code);
                    $default
                }
                Err(error) => {
                    set_runtime_error(error.to_string());
                    set_trap(TrapCode::HostTrap);
                    $default
                }
            }
        }
    };
}

macro_rules! define_shared_store {
    ($name:ident, $ty:ty, $call:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(mapping_id: u64, offset: u32, value: $ty) {
            let mapping_id = SharedMemoryMappingId::from_raw(mapping_id);
            let Some(result) =
                with_current_module_mut(|module| $call(module, mapping_id, offset, value))
            else {
                set_runtime_error("missing JIT execution context for shared memory".to_string());
                set_trap(TrapCode::HostTrap);
                return;
            };

            match result {
                Ok(()) => {}
                Err(WasmError::Trap(code)) => set_trap(code),
                Err(error) => {
                    set_runtime_error(error.to_string());
                    set_trap(TrapCode::HostTrap);
                }
            }
        }
    };
}

define_shared_load!(
    llvm_jit_shared_memory_i32_load,
    i32,
    0i32,
    |module, mapping_id, offset| { module.read_shared_region_i32(mapping_id, offset) }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load,
    i64,
    0i64,
    |module, mapping_id, offset| { module.read_shared_region_i64(mapping_id, offset) }
);
define_shared_load!(
    llvm_jit_shared_memory_f32_load,
    f32,
    0.0f32,
    |module, mapping_id, offset| { module.read_shared_region_f32(mapping_id, offset) }
);
define_shared_load!(
    llvm_jit_shared_memory_f64_load,
    f64,
    0.0f64,
    |module, mapping_id, offset| { module.read_shared_region_f64(mapping_id, offset) }
);
define_shared_load!(
    llvm_jit_shared_memory_i32_load8_s,
    i32,
    0i32,
    |module, mapping_id, offset| {
        module
            .read_shared_region_u8(mapping_id, offset)
            .map(|v| v as i8 as i32)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i32_load8_u,
    i32,
    0i32,
    |module, mapping_id, offset| {
        module
            .read_shared_region_u8(mapping_id, offset)
            .map(|v| v as i32)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i32_load16_s,
    i32,
    0i32,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 2];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(i16::from_le_bytes(bytes) as i32)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i32_load16_u,
    i32,
    0i32,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 2];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(u16::from_le_bytes(bytes) as i32)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load8_s,
    i64,
    0i64,
    |module, mapping_id, offset| {
        module
            .read_shared_region_u8(mapping_id, offset)
            .map(|v| v as i8 as i64)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load8_u,
    i64,
    0i64,
    |module, mapping_id, offset| {
        module
            .read_shared_region_u8(mapping_id, offset)
            .map(|v| v as i64)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load16_s,
    i64,
    0i64,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 2];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(i16::from_le_bytes(bytes) as i64)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load16_u,
    i64,
    0i64,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 2];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(u16::from_le_bytes(bytes) as i64)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load32_s,
    i64,
    0i64,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 4];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(i32::from_le_bytes(bytes) as i64)
    }
);
define_shared_load!(
    llvm_jit_shared_memory_i64_load32_u,
    i64,
    0i64,
    |module, mapping_id, offset| {
        let mut bytes = [0u8; 4];
        module.read_shared_region(mapping_id, offset, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes) as i64)
    }
);

define_shared_store!(
    llvm_jit_shared_memory_i32_store,
    i32,
    |module, mapping_id, offset, value| {
        module.write_shared_region_i32(mapping_id, offset, value)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i64_store,
    i64,
    |module, mapping_id, offset, value| {
        module.write_shared_region_i64(mapping_id, offset, value)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_f32_store,
    f32,
    |module, mapping_id, offset, value| {
        module.write_shared_region_f32(mapping_id, offset, value)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_f64_store,
    f64,
    |module, mapping_id, offset, value| {
        module.write_shared_region_f64(mapping_id, offset, value)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i32_store8,
    i32,
    |module, mapping_id, offset, value| {
        module.write_shared_region_u8(mapping_id, offset, value as u8)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i32_store16,
    i32,
    |module, mapping_id, offset, value| {
        module.write_shared_region(mapping_id, offset, &(value as u16).to_le_bytes())
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i64_store8,
    i64,
    |module, mapping_id, offset, value| {
        module.write_shared_region_u8(mapping_id, offset, value as u8)
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i64_store16,
    i64,
    |module, mapping_id, offset, value| {
        module.write_shared_region(mapping_id, offset, &(value as u16).to_le_bytes())
    }
);
define_shared_store!(
    llvm_jit_shared_memory_i64_store32,
    i64,
    |module, mapping_id, offset, value| {
        module.write_shared_region(mapping_id, offset, &(value as u32).to_le_bytes())
    }
);

macro_rules! define_load {
    ($name:ident, $size:expr, $ty:ty, $default:expr, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(addr: u32) -> $ty {
            match check_bounds(addr, $size) {
                Some(ptr) => unsafe { $body(ptr) },
                None => {
                    set_trap(TrapCode::MemoryOutOfBounds);
                    $default
                }
            }
        }
    };
}

macro_rules! define_store {
    ($name:ident, $size:expr, $ty:ty, $body:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $name(addr: u32, val: $ty) {
            match check_bounds(addr, $size) {
                Some(ptr) => unsafe { $body(ptr, val) },
                None => set_trap(TrapCode::MemoryOutOfBounds),
            }
        }
    };
}

define_load!(
    llvm_jit_i32_load,
    4,
    i32,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i32)
);
define_load!(
    llvm_jit_i64_load,
    8,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i64)
);
define_load!(llvm_jit_f32_load, 4, f32, 0.0, |ptr| {
    std::ptr::read_unaligned(ptr as *const f32)
});
define_load!(llvm_jit_f64_load, 8, f64, 0.0, |ptr| {
    std::ptr::read_unaligned(ptr as *const f64)
});
define_load!(
    llvm_jit_i32_load8_s,
    1,
    i32,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i8) as i32
);
define_load!(
    llvm_jit_i32_load8_u,
    1,
    i32,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const u8) as i32
);
define_load!(
    llvm_jit_i32_load16_s,
    2,
    i32,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i16) as i32
);
define_load!(
    llvm_jit_i32_load16_u,
    2,
    i32,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const u16) as i32
);
define_load!(
    llvm_jit_i64_load8_s,
    1,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i8) as i64
);
define_load!(
    llvm_jit_i64_load8_u,
    1,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const u8) as i64
);
define_load!(
    llvm_jit_i64_load16_s,
    2,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i16) as i64
);
define_load!(
    llvm_jit_i64_load16_u,
    2,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const u16) as i64
);
define_load!(
    llvm_jit_i64_load32_s,
    4,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const i32) as i64
);
define_load!(
    llvm_jit_i64_load32_u,
    4,
    i64,
    0,
    |ptr| std::ptr::read_unaligned(ptr as *const u32) as i64
);

define_store!(llvm_jit_i32_store, 4, i32, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut i32, val)
});
define_store!(llvm_jit_i64_store, 8, i64, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut i64, val)
});
define_store!(llvm_jit_f32_store, 4, f32, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut f32, val)
});
define_store!(llvm_jit_f64_store, 8, f64, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut f64, val)
});
define_store!(llvm_jit_i32_store8, 1, i32, |ptr, val| {
    std::ptr::write_unaligned(ptr, val as u8)
});
define_store!(llvm_jit_i32_store16, 2, i32, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut u16, val as u16)
});
define_store!(llvm_jit_i64_store8, 1, i64, |ptr, val| {
    std::ptr::write_unaligned(ptr, val as u8)
});
define_store!(llvm_jit_i64_store16, 2, i64, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut u16, val as u16)
});
define_store!(llvm_jit_i64_store32, 4, i64, |ptr, val| {
    std::ptr::write_unaligned(ptr as *mut u32, val as u32)
});

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i32_div_s(a: i32, b: i32) -> i32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    if a == i32::MIN && b == -1 {
        set_trap(TrapCode::IntegerOverflow);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i32_div_u(a: u32, b: u32) -> u32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i32_rem_s(a: i32, b: i32) -> i32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i32_rem_u(a: u32, b: u32) -> u32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i64_div_s(a: i64, b: i64) -> i64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    if a == i64::MIN && b == -1 {
        set_trap(TrapCode::IntegerOverflow);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i64_div_u(a: u64, b: u64) -> u64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i64_rem_s(a: i64, b: i64) -> i64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_i64_rem_u(a: u64, b: u64) -> u64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_f32_min(a: f32, b: f32) -> f32 {
    wasm_f32_min(a, b)
}

#[unsafe(no_mangle)]
#[allow(missing_docs)]
pub extern "C" fn llvm_jit_f64_min(a: f64, b: f64) -> f64 {
    wasm_f64_min(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_checking() {
        let mut mem = vec![0u8; 1024];
        mem[0] = 42;
        mem[1] = 0;
        mem[2] = 0;
        mem[3] = 0;

        set_memory_context(mem.as_mut_ptr(), 1024);
        clear_trap();

        let val = llvm_jit_i32_load(0);
        assert_eq!(val, 42);
        assert_eq!(take_trap(), None);

        llvm_jit_i32_store(0, 100);
        let val = llvm_jit_i32_load(0);
        assert_eq!(val, 100);
        assert_eq!(take_trap(), None);
    }

    #[test]
    fn test_out_of_bounds_sets_trap() {
        let mut mem = vec![0u8; 4];
        set_memory_context(mem.as_mut_ptr(), mem.len());
        clear_trap();

        let _ = llvm_jit_i64_load(0);
        assert_eq!(take_trap(), Some(TrapCode::MemoryOutOfBounds));
    }

    #[test]
    fn test_meter_tick_without_execution_context_sets_host_trap() {
        clear_trap();
        set_memory_context(std::ptr::null_mut(), 0);

        llvm_jit_meter_tick(1);

        assert_eq!(take_trap(), Some(TrapCode::HostTrap));
        assert_eq!(
            take_runtime_error(),
            Some("missing JIT execution context for metering".to_string())
        );
    }

    #[test]
    fn test_memory_grow_internal_error_surfaces_runtime_error() {
        let mut module = AotModule::from_module(&crate::runtime::Module::new());
        clear_trap();
        unsafe { set_execution_context(&mut module, std::ptr::null_mut(), 0, Some(1)) }.unwrap();

        let result = llvm_jit_memory_grow(1);

        assert_eq!(result, -1);
        assert!(take_runtime_error().unwrap().contains("memory not found"));
        assert_eq!(take_trap(), Some(TrapCode::HostTrap));
        assert!(clear_execution_context_for_owner(1, true));
    }

    #[test]
    fn test_shared_memory_helpers_access_attached_region() {
        let mut module = AotModule::from_module(&crate::runtime::Module::new());
        let region_id = module.allocate_shared_region(16, 4).unwrap();
        let mapping_id = module.attach_shared_region_whole(region_id).unwrap();

        clear_trap();
        unsafe { set_execution_context(&mut module, std::ptr::null_mut(), 0, Some(1)) }.unwrap();

        llvm_jit_shared_memory_i32_store(mapping_id.raw(), 4, 123);
        assert_eq!(take_trap(), None);
        assert_eq!(module.read_shared_region_i32(mapping_id, 4).unwrap(), 123);

        let value = llvm_jit_shared_memory_i32_load(mapping_id.raw(), 4);
        assert_eq!(value, 123);
        assert_eq!(take_trap(), None);
        assert!(clear_execution_context_for_owner(1, true));
    }

    #[test]
    fn test_shared_memory_helper_reports_detached_mapping() {
        let mut module = AotModule::from_module(&crate::runtime::Module::new());
        let region_id = module.allocate_shared_region(16, 4).unwrap();
        let mapping_id = module.attach_shared_region_whole(region_id).unwrap();
        module.detach_shared_region(mapping_id).unwrap();

        clear_trap();
        unsafe { set_execution_context(&mut module, std::ptr::null_mut(), 0, Some(1)) }.unwrap();

        let value = llvm_jit_shared_memory_i32_load(mapping_id.raw(), 0);

        assert_eq!(value, 0);
        assert_eq!(take_trap(), Some(TrapCode::HostTrap));
        assert!(
            take_runtime_error()
                .unwrap()
                .contains("detached or not attached")
        );
        assert!(clear_execution_context_for_owner(1, true));
    }
}
