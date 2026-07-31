## REMOVED Requirements

### Requirement: LLVM ORC JIT integration
**Reason**: The sole consumer (Selium) builds wasmtiny with `default-features = false` and never enables `llvm-jit`; the LLVM dependency and its ~108 `unsafe` blocks are unjustified supply-chain and maintenance risk for an unused engine.
**Migration**: All execution uses the classic interpreter; the `llvm-jit` cargo feature no longer exists.

### Requirement: LLVM optimization passes
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Code generation from LLVM IR
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Symbol resolution for imports
**Reason**: Removed with the LLVM JIT; imports resolve to `HostFunc` implementations in the interpreter.
**Migration**: Register host functions via `WasmApplication::register_host_function`.

### Requirement: Error handling for LLVM compilation failures
**Reason**: Removed with the LLVM JIT; there is no compilation step whose failure needs a fallback path.
**Migration**: Module load/validation errors are returned from the loader directly.
