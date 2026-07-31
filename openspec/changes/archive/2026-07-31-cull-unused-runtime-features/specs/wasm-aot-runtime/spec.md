## REMOVED Requirements

### Requirement: AOT module loading
**Reason**: No AOT compilation exists or is planned; the module named `aot_runtime` was the interpreter-backed core engine and is renamed accordingly (see `wasm-runtime-core`). The sole consumer (Selium) never references AOT types.
**Migration**: Use the `engine` module's loader (`WasmApplication::load_module_from_memory` path).

### Requirement: Native function execution
**Reason**: There is no native-compiled WASM execution; execution is interpreter-only. The `NativeFunc`/`call_native` registration concept had no execution-path consumer.
**Migration**: Host interaction uses imported `HostFunc` functions called by the interpreter.

### Requirement: Call frame management
**Reason**: Was specified for a native AOT calling convention that does not exist; interpreter frames are an internal implementation detail of the interpreter.
**Migration**: No embedder action; frame management is internal to the interpreter.

### Requirement: Memory management
**Reason**: AOT-specific native memory management does not exist; linear memory is provided by the mmap-backed memory used by the interpreter (see `mmap-backed-memory`).
**Migration**: Use engine/interpreter memory APIs (`Memory`, `SharedMemoryRegistry`).

### Requirement: Table management
**Reason**: AOT-specific; tables are managed by the interpreter-backed engine (see `wasm-runtime-core`).
**Migration**: No embedder action.

### Requirement: Global variable access
**Reason**: AOT-specific; globals are managed by the interpreter-backed engine (see `wasm-runtime-core`).
**Migration**: No embedder action.

### Requirement: Intrinsic function support
**Reason**: AOT-specific native intrinsics do not exist; numeric operations are interpreter instructions.
**Migration**: No embedder action.
