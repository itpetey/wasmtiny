use std::cell::RefCell;

thread_local! {
    static LLVM_RUNTIME_CTX: RefCell<LlvmRuntimeContext> = const { RefCell::new(LlvmRuntimeContext::new()) };
}

struct LlvmRuntimeContext {
    memory_ptr: *mut u8,
    memory_len: u32,
}

impl LlvmRuntimeContext {
    const fn new() -> Self {
        Self {
            memory_ptr: std::ptr::null_mut(),
            memory_len: 0,
        }
    }
}

pub fn set_memory_context(ptr: *mut u8, len: u32) {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.memory_ptr = ptr;
        ctx.memory_len = len;
    });
}

fn check_bounds(addr: u32, size: u32) -> Result<*mut u8, ()> {
    LLVM_RUNTIME_CTX.with(|ctx| {
        let ctx = ctx.borrow();
        if ctx.memory_ptr.is_null() {
            return Err(());
        }
        let end = addr.checked_add(size).ok_or(())?;
        if end > ctx.memory_len {
            return Err(());
        }
        Ok(unsafe { ctx.memory_ptr.add(addr as usize) })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_load(addr: u32) -> i32 {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe { std::ptr::read_unaligned(ptr as *const i32) },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load(addr: u32) -> i64 {
    match check_bounds(addr, 8) {
        Ok(ptr) => unsafe { std::ptr::read_unaligned(ptr as *const i64) },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_f32_load(addr: u32) -> f32 {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe { std::ptr::read_unaligned(ptr as *const f32) },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_f64_load(addr: u32) -> f64 {
    match check_bounds(addr, 8) {
        Ok(ptr) => unsafe { std::ptr::read_unaligned(ptr as *const f64) },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_load8_s(addr: u32) -> i32 {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const i8);
            val as i32
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_load8_u(addr: u32) -> i32 {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const u8);
            val as i32
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_load16_s(addr: u32) -> i32 {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const i16);
            val as i32
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_load16_u(addr: u32) -> i32 {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const u16);
            val as i32
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load8_s(addr: u32) -> i64 {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const i8);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load8_u(addr: u32) -> i64 {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const u8);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load16_s(addr: u32) -> i64 {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const i16);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load16_u(addr: u32) -> i64 {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const u16);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load32_s(addr: u32) -> i64 {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const i32);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_load32_u(addr: u32) -> i64 {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe {
            let val = std::ptr::read_unaligned(ptr as *const u32);
            val as i64
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_store(addr: u32, val: i32) {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut i32, val);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_store(addr: u32, val: i64) {
    match check_bounds(addr, 8) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut i64, val);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_f32_store(addr: u32, val: f32) {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut f32, val);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_f64_store(addr: u32, val: f64) {
    match check_bounds(addr, 8) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut f64, val);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_store8(addr: u32, val: i32) {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr, val as u8);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i32_store16(addr: u32, val: i32) {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut u16, val as u16);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_store8(addr: u32, val: i64) {
    match check_bounds(addr, 1) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr, val as u8);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_store16(addr: u32, val: i64) {
    match check_bounds(addr, 2) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut u16, val as u16);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn llvm_jit_i64_store32(addr: u32, val: i64) {
    match check_bounds(addr, 4) {
        Ok(ptr) => unsafe {
            std::ptr::write_unaligned(ptr as *mut u32, val as u32);
        },
        Err(_) => {
            eprintln!("LLVM JIT: Memory access out of bounds at address {}", addr);
            std::process::abort();
        }
    }
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

        let val = llvm_jit_i32_load(0);
        assert_eq!(val, 42);

        llvm_jit_i32_store(0, 100);
        let val = llvm_jit_i32_load(0);
        assert_eq!(val, 100);
    }
}
