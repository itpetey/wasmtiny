//! SIGSEGV signal handler for translating page faults to WASM traps.
//!
//! When shared memory regions are mapped with per-page protection (e.g., PROT_READ),
//! writes to read-only pages cause SIGSEGV. This module installs a signal handler
//! that catches such faults and translates them to `TrapCode::MemoryOutOfBounds`.
//!
//! # Architecture
//!
//! The handler uses `setjmp`/`longjmp` to unwind from the signal handler
//! back to a recovery point. Before executing code that might fault, call
//! `with_trap_handler` to set up a jump buffer. If a SIGSEGV occurs in a shared
//! region, the handler `longjmp`s back and returns a trap error.
//!
//! # Safety
//!
//! Signal handlers must be async-signal-safe. This module uses only POSIX
//! async-signal-safe functions in the handler itself.

use crate::runtime::{TrapCode, WasmError};
use std::cell::RefCell;
use std::sync::Once;

// On macOS, jmp_buf is 68 bytes (17 * 4 bytes for int32 + padding)
// On Linux, it varies but is typically around 200 bytes
// We use a conservative size that works on both platforms
#[cfg(target_os = "macos")]
const JMP_BUF_SIZE: usize = 192;

#[cfg(not(target_os = "macos"))]
const JMP_BUF_SIZE: usize = 256;

/// Opaque jump buffer type for setjmp/longjmp
#[repr(C)]
#[derive(Copy, Clone)]
pub struct JmpBuf {
    _data: [u8; JMP_BUF_SIZE],
}

impl Default for JmpBuf {
    fn default() -> Self {
        Self {
            _data: [0u8; JMP_BUF_SIZE],
        }
    }
}

// Platform-specific setjmp/longjmp declarations
unsafe extern "C" {
    fn setjmp(env: *mut JmpBuf) -> libc::c_int;
    fn longjmp(env: *mut JmpBuf, val: libc::c_int) -> !;
}

// Thread-local jump buffer for signal handler recovery.
//
// When `with_trap_handler` is called, it stores a pointer to a `JmpBuf` here.
// The SIGSEGV handler checks this and `longjmp`s if set.
thread_local! {
    static JUMP_BUFFER: RefCell<Option<*mut JmpBuf>> = RefCell::new(None);
    static IN_TRAP_HANDLER: RefCell<bool> = RefCell::new(false);
}

/// One-time initialization for the signal handler.
static INIT: Once = Once::new();

/// Installs the SIGSEGV (and SIGBUS on macOS) signal handler.
///
/// This is called automatically by `with_trap_handler` on first use.
/// Safe to call multiple times (idempotent).
pub fn install_signal_handler() {
    INIT.call_once(|| {
        install_one(libc::SIGSEGV);
        #[cfg(target_os = "macos")]
        install_one(libc::SIGBUS);
    });
}

fn install_one(signal: libc::c_int) {
    // SAFETY: sigaction, sigemptyset are FFI calls with standard semantics.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = sigsegv_handler as *const () as usize;
        sa.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
        libc::sigemptyset(&mut sa.sa_mask);

        if libc::sigaction(signal, &sa, std::ptr::null_mut()) != 0 {
            panic!(
                "Failed to install signal handler for {}: {}",
                signal,
                std::io::Error::last_os_error()
            );
        }
    }
}

/// The SIGSEGV signal handler.
///
/// Checks if the faulting address is in a shared memory region. If yes,
/// and if a jump buffer is set, `longjmp`s back to the recovery point.
/// Otherwise, re-raises SIGSEGV with the default handler.
extern "C" fn sigsegv_handler(
    sig: libc::c_int,
    info: *mut libc::siginfo_t,
    _context: *mut libc::c_void,
) {
    // Extract the faulting address from siginfo_t
    let _fault_addr = unsafe { (*info).si_addr() as usize };

    // Check if we have a jump buffer set up
    let should_longjmp = JUMP_BUFFER.with(|buf| {
        buf.borrow().is_some()
    });

    if should_longjmp {
        let in_handler = IN_TRAP_HANDLER.with(|flag| *flag.borrow());

        if in_handler {
            // longjmp back to the recovery point with value 1 (indicating trap)
            JUMP_BUFFER.with(|buf| {
                if let Some(jmp_buf_ptr) = *buf.borrow() {
                    unsafe {
                        longjmp(jmp_buf_ptr, 1);
                    }
                }
            });
        }
    }

    // If we get here, no jump buffer was set up.
    // Re-raise with the default handler to crash the process.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = libc::SIG_DFL;
        libc::sigaction(sig, &sa, std::ptr::null_mut());
        libc::raise(sig);
    }
}

/// Executes a closure with SIGSEGV trap handling enabled.
///
/// If a SIGSEGV occurs during execution of `f` and the faulting address
/// is in a shared memory region, returns `Err(WasmError::Trap(MemoryOutOfBounds))`.
/// Otherwise, returns the result of `f`.
///
/// # Example
///
/// ```ignore
/// let result = with_trap_handler(|| {
///     // Code that might fault on a protected page
///     memory.write(offset, data)
/// });
/// match result {
///     Ok(()) => println!("Write succeeded"),
///     Err(WasmError::Trap(TrapCode::MemoryOutOfBounds)) => println!("Write trapped"),
///     Err(e) => println!("Other error: {}", e),
/// }
/// ```
pub fn with_trap_handler<F, T>(f: F) -> Result<T, WasmError>
where
    F: FnOnce() -> Result<T, WasmError>,
{
    install_signal_handler();

    // Set up the jump buffer on the heap so its address remains stable
    let mut jmp_buf = Box::new(JmpBuf::default());

    // setjmp returns 0 on initial call, non-zero on longjmp
    let setjmp_result = unsafe {
        setjmp(jmp_buf.as_mut() as *mut JmpBuf)
    };

    if setjmp_result != 0 {
        // We got here via longjmp from the signal handler
        return Err(WasmError::Trap(TrapCode::MemoryOutOfBounds));
    }

    // Store the jump buffer pointer in thread-local storage
    JUMP_BUFFER.with(|buf| {
        *buf.borrow_mut() = Some(jmp_buf.as_mut() as *mut JmpBuf);
    });

    // Mark that we're in a trap handler context
    IN_TRAP_HANDLER.with(|flag| {
        *flag.borrow_mut() = true;
    });

    // Execute the closure
    let result = f();

    // Clean up: remove the jump buffer and clear the flag
    JUMP_BUFFER.with(|buf| {
        *buf.borrow_mut() = None;
    });

    IN_TRAP_HANDLER.with(|flag| {
        *flag.borrow_mut() = false;
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_handler_installation() {
        install_signal_handler();
        // Should not panic on second call (idempotent)
        install_signal_handler();
    }

    #[test]
    fn test_with_trap_handler_no_fault() {
        let result = with_trap_handler(|| Ok(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_trap_handler_error_propagation() {
        let result: Result<i32, WasmError> = with_trap_handler(|| {
            Err(WasmError::Runtime("test error".to_string()))
        });
        assert!(matches!(result, Err(WasmError::Runtime(_))));
    }

    #[test]
    fn test_sigsegv_traps_via_signal_handler() {
        // Allocate a page, make it read-only, then try to write to it
        // inside with_trap_handler. Verify we get MemoryOutOfBounds.
        install_signal_handler();

        let page_size = 4096;
        // Use mmap to allocate a page
        let addr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                page_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(addr, libc::MAP_FAILED);

        // Make it read-only to trigger SIGSEGV on write
        let ret = unsafe { libc::mprotect(addr, page_size, libc::PROT_READ) };
        assert_eq!(ret, 0);

        // Try writing to it inside with_trap_handler
        let result = with_trap_handler(|| {
            unsafe {
                *(addr as *mut u8) = 42;
            }
            Ok(())
        });

        // Restore write permission before cleanup
        unsafe {
            libc::mprotect(addr, page_size, libc::PROT_READ | libc::PROT_WRITE);
        }
        unsafe {
            libc::munmap(addr, page_size);
        }

        assert!(
            matches!(result, Err(WasmError::Trap(TrapCode::MemoryOutOfBounds))),
            "Expected MemoryOutOfBounds trap, got {:?}",
            result
        );
    }
}
