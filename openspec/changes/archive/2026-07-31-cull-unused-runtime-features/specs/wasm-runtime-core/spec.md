## ADDED Requirements

### Requirement: Accurately named core engine
The crate SHALL expose its core runtime (module loading, instantiation, invocation) under an `engine` module with type names reflecting its interpreter-backed nature. No public item SHALL use `Aot`/`aot_runtime` naming, and no ahead-of-time compilation pipeline SHALL exist.

#### Scenario: No AOT-named public API
- **WHEN** the crate's public API is inspected
- **THEN** there SHALL be no `aot_runtime` module and no `AotRuntime`/`AotModule`/`AotLoader`/`AotExport` types; their roles SHALL be provided by accurately named equivalents under `engine`

#### Scenario: No native-symbol concept
- **WHEN** the engine API is inspected
- **THEN** there SHALL be no `NativeFunc`/`native_functions`/`call_native` registration concept; host interaction SHALL occur exclusively through imported `HostFunc` functions

### Requirement: Consumer-driven public API surface
Public API items SHALL exist only where they serve the interpreter-based embedder use case (module loading, host-function registration, instantiation, invocation, shared-region management, memory access). Items without any caller in the crate or its known embedder SHALL be removed rather than retained.

#### Scenario: No dead convenience methods
- **WHEN** the public APIs of `engine`, `runtime`, and `application` modules are audited for callers
- **THEN** every public method SHALL have at least one caller in the crate, the test suites, or the known embedder (Selium)

## REMOVED Requirements

### Requirement: Function invocation
**Reason**: Replaced by a requirement that reflects the actual invocation path (via the engine/application, not direct `Instance::call`).
**Migration**: Invoke exported functions through `WasmApplication::call_function` (or the engine equivalent); direct `Instance` manipulation is internal.

## MODIFIED Requirements

### Requirement: Instance creation
The runtime SHALL allow instantiation of a module into an `Instance` with isolated linear memory and table spaces. Instance construction and binding SHALL be managed by the core engine; per-invocation instance state SHALL be cached and reused across calls to the same loaded module rather than rebuilt from a cloned module.

#### Scenario: Instantiation through the engine
- **WHEN** a loaded module is instantiated via `WasmApplication::instantiate`
- **THEN** an instance with isolated linear memory and table spaces is created and associated with that loaded module
