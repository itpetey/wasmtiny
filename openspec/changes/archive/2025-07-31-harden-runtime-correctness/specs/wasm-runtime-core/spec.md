## ADDED Requirements

### Requirement: Bounded per-invocation engine cost
Calling an exported function on an already-instantiated module SHALL NOT deep-clone the module, SHALL reuse the module's instance state, and SHALL NOT permanently grow any engine registry (funcref store, native table) as a function of call count.

#### Scenario: Repeated calls do not grow the store
- **WHEN** an exported function is called N times on the same loaded module
- **THEN** engine registry sizes (e.g. the funcref store) SHALL be the same after the first call as after the Nth

#### Scenario: Repeated calls reuse instance state
- **WHEN** an exported function mutates memory or globals and is called again later
- **THEN** the second call SHALL observe the prior call's mutations (state persists across invocations of the same loaded module)

### Requirement: Value codec round-trip fidelity
`WasmValue` byte serialisation (`to_bytes`/`from_bytes`) SHALL round-trip every representable variant exactly, using self-consistent type tags.

#### Scenario: NullRef round-trip
- **WHEN** `WasmValue::NullRef(RefType::ExternRef)` is serialised and deserialised
- **THEN** the result SHALL equal the original value (reference kind preserved)

#### Scenario: All-variant round-trip
- **WHEN** any `WasmValue` (I32, I64, F32, F64, FuncRef, ExternRef, NullRef of either kind) is serialised and deserialised
- **THEN** the result SHALL equal the original value

### Requirement: Callback-safe lock discipline
The engine SHALL NOT hold any store, instance, memory, or registry lock across a call into embedder-provided code (`HostFunc` implementations), and lock acquisition order across these objects SHALL follow a single global order to prevent ABBA deadlock.

#### Scenario: Host callback may re-enter engine APIs
- **WHEN** a `HostFunc` implementation calls back into engine APIs that acquire store or registry locks
- **THEN** the call SHALL complete without deadlock

#### Scenario: Concurrent attach and instance drop
- **WHEN** one thread attaches/detaches shared regions on a memory while another thread drops an instance sharing that memory and registry
- **THEN** both operations SHALL complete without deadlock and attachment accounting SHALL remain consistent

## MODIFIED Requirements

### Requirement: Cross-module import aliasing
The runtime SHALL preserve shared state for imported guest functions, tables, memories, and globals across module boundaries. Imported tables SHALL be shared by reference (mutations visible to all importers), and nested instantiation for imported guest functions SHALL share the caller's store (native registry and shared-memory registry).

#### Scenario: Imported table aliases exported table state
- **GIVEN** module A exports a table and module B imports that table
- **WHEN** module B mutates the imported table contents (including through a guest-function callback)
- **THEN** subsequent reads through module A SHALL observe the same table contents

#### Scenario: Imported guest function binding executes real guest code
- **GIVEN** module A exports a WebAssembly function and module B imports it
- **WHEN** module B calls the imported function directly or through a funcref stored in a table
- **THEN** the exported WebAssembly function body from module A SHALL execute with the correct type checks and results, with access to the same store state as the caller

### Requirement: Error handling
The runtime SHALL use `Result<T, WasmError>` for all fallible operations with structured error types. `WasmError` SHALL use structured, typed variants (via `thiserror`) rather than free-form string payloads where variant data has known shape; variants constructed or matched by known embedders (`Runtime`, `Instantiate`) SHALL remain constructible/matchable with compatible shapes or be migrated with the embedder.

#### Scenario: Errors carry typed information
- **WHEN** a trap or validation failure is returned
- **THEN** the error value SHALL expose its kind programmatically (variant + typed fields), not solely via message text

#### Scenario: Out of bounds memory access
- **WHEN** a WASM module attempts to read memory at an offset beyond allocation
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`
