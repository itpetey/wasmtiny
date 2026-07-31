## REMOVED Requirements

### Requirement: WASM to LLVM IR translation
**Reason**: The sole consumer never enables `llvm-jit`; the translator existed only to feed the removed LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Type mapping
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Local variable handling
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Stack value management
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.

### Requirement: Function call translation
**Reason**: Removed with the LLVM JIT.
**Migration**: All execution uses the classic interpreter.
