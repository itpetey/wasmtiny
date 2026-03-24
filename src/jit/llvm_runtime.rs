use crate::aot_runtime::runtime::AotModule;
use crate::runtime::{NumType, TrapCode, ValType, WasmError, WasmValue};
use std::cell::RefCell;
use std::ptr;

thread_local! {
    static LLVM_RUNTIME_CTX: RefCell<LlvmRuntimeContext> =
        const { RefCell::new(LlvmRuntimeContext::new()) };
}

struct LlvmRuntimeContext {
    memory_ptr: *mut u8,
    memory_len: usize,
    current_module: *mut AotModule,
    trap: Option<TrapCode>,
}

impl LlvmRuntimeContext {
    const fn new() -> Self {
        Self {
            memory_ptr: ptr::null_mut(),
            memory_len: 0,
            current_module: ptr::null_mut(),
            trap: None,
        }
    }
}

/// Sets the execution context for the current thread.
///
/// This must be called before invoking compiled WASM code so runtime helpers can
/// reach the active module, linear memory, and trap state.
pub fn set_execution_context(module: *mut AotModule, memory_ptr: *mut u8, memory_len: usize) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.current_module = module;
        ctx.memory_ptr = memory_ptr;
        ctx.memory_len = memory_len;
        ctx.trap = None;
    });
}

/// Sets only the memory context for the current thread.
///
/// This is kept for compatibility with existing tests and callers that do not
/// need module-backed import dispatch.
pub fn set_memory_context(ptr: *mut u8, len: usize) {
    set_execution_context(ptr::null_mut(), ptr, len);
}

pub fn clear_trap() {
    LLVM_RUNTIME_CTX.with(|ctx| {
        ctx.borrow_mut().trap = None;
    });
}

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
        TrapCode::TableOutOfBounds => 3,
        TrapCode::IndirectCallTypeMismatch => 4,
        TrapCode::StackOverflow => 5,
        TrapCode::IntegerOverflow => 6,
        TrapCode::IntegerDivisionByZero => 7,
        TrapCode::InvalidConversionToInt => 8,
        TrapCode::CallIndirectNull => 9,
        TrapCode::NullReference => 10,
        TrapCode::HostTrap => 11,
    }
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
pub extern "C" fn llvm_jit_trap_unreachable() {
    set_trap(TrapCode::Unreachable);
}

#[unsafe(no_mangle)]
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

        module.invoke_function(func_idx, &args)
    }) else {
        set_trap(TrapCode::HostTrap);
        return;
    };

    match result {
        Ok(results) => {
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
        Err(WasmError::Trap(code)) => set_trap(code),
        Err(_) => set_trap(TrapCode::HostTrap),
    }

    refresh_memory_context_from_module();
}

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
pub extern "C" fn llvm_jit_i32_div_u(a: u32, b: u32) -> u32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_rem_s(a: i32, b: i32) -> i32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_rem_u(a: u32, b: u32) -> u32 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
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
pub extern "C" fn llvm_jit_i64_div_u(a: u64, b: u64) -> u64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_rem_s(a: i64, b: i64) -> i64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_rem_u(a: u64, b: u64) -> u64 {
    if b == 0 {
        set_trap(TrapCode::IntegerDivisionByZero);
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_f32_min(a: f32, b: f32) -> f32 {
    wasm_f32_min(a, b)
}

#[unsafe(no_mangle)]
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
}
