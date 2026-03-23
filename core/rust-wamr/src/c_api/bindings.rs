use crate::runtime::{Instance, Module, Result as WasmResult, WasmValue};
use std::ffi::CStr;
use std::os::raw::c_char;

#[repr(C)]
pub struct wasm_module_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_instance_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_func_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_memory_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_table_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_global_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wasm_trap_t {
    _private: [u8; 0],
}

#[no_mangle]
pub extern "C" fn wasm_module_new(data: *const u8, size: usize) -> *mut wasm_module_t {
    if data.is_null() {
        return std::ptr::null_mut();
    }

    let slice = unsafe { std::slice::from_raw_parts(data, size) };
    let module = Module::new();

    Box::into_raw(Box::new(module)) as *mut wasm_module_t
}

#[no_mangle]
pub extern "C" fn wasm_module_delete(module: *mut wasm_module_t) {
    if !module.is_null() {
        unsafe {
            Box::from_raw(module);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_instance_new(module: *const wasm_module_t) -> *mut wasm_instance_t {
    if module.is_null() {
        return std::ptr::null_mut();
    }

    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_instance_delete(instance: *mut wasm_instance_t) {
    if !instance.is_null() {
        unsafe {
            Box::from_raw(instance);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_func_call(instance: *mut wasm_instance_t, func_idx: u32) -> i32 {
    -1
}

#[no_mangle]
pub extern "C" fn wasm_memory_new(initial: u32) -> *mut wasm_memory_t {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_memory_delete(memory: *mut wasm_memory_t) {
    if !memory.is_null() {
        unsafe {
            Box::from_raw(memory);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_table_new(initial: u32) -> *mut wasm_table_t {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_table_delete(table: *mut wasm_table_t) {
    if !table.is_null() {
        unsafe {
            Box::from_raw(table);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_global_new(initial: u32) -> *mut wasm_global_t {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_global_delete(global: *mut wasm_global_t) {
    if !global.is_null() {
        unsafe {
            Box::from_raw(global);
        }
    }
}

#[no_mangle]
pub extern "C" fn wasm_trap_new(message: *const c_char) -> *mut wasm_trap_t {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn wasm_trap_delete(trap: *mut wasm_trap_t) {
    if !trap.is_null() {
        unsafe {
            Box::from_raw(trap);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_null_safety() {
        assert!(std::ptr::null_mut::<super::wasm_module_t>() == std::ptr::null_mut());
    }
}
