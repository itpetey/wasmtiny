## REMOVED Requirements

### Requirement: Execute LLVM-compiled code
**Reason**: The sole consumer never enables `llvm-jit`; the execution infrastructure for LLVM-compiled code is removed with the compiler.
**Migration**: All execution uses the classic interpreter.

### Requirement: Memory access from LLVM-compiled code
**Reason**: Removed with the LLVM JIT; memory access helpers exist only in the interpreter.
**Migration**: All execution uses the classic interpreter.

### Requirement: Stack frame management for LLVM code
**Reason**: Removed with the LLVM JIT.
**Migration**: No embedder action; frames are internal to the interpreter.

### Requirement: Integration with WasmApplication
**Reason**: Removed with the LLVM JIT; `ExecutionMode` selection no longer exists.
**Migration**: `WasmApplication` always executes via the interpreter; remove any `set_execution_mode` calls.

### Requirement: Host function integration
**Reason**: Removed with the LLVM JIT; host functions are called by the interpreter directly.
**Migration**: Register host functions via `WasmApplication::register_host_function`.
