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

### Requirement: Module initialization
The runtime SHALL provide a `Module` struct representing a loaded WASM module with types, functions, memories, tables, globals, and exports.

### Requirement: Instance creation
The runtime SHALL allow instantiation of a module into an `Instance` with isolated linear memory and table spaces. Instance construction and binding SHALL be managed by the core engine; per-invocation instance state SHALL be cached and reused across calls to the same loaded module rather than rebuilt from a cloned module.

#### Scenario: Instantiation through the engine
- **WHEN** a loaded module is instantiated via `WasmApplication::instantiate`
- **THEN** an instance with isolated linear memory and table spaces is created and associated with that loaded module

### Requirement: Memory access
The runtime SHALL provide safe read/write access to linear memory with bounds checking. Memory access SHALL include both owned pages and mapped shared region pages. Writes to read-only shared pages SHALL trap before reaching memory.

#### Scenario: Out of bounds memory access
- **WHEN** a WASM module attempts to read memory at an offset beyond allocation
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Errors carry typed information
- **WHEN** a trap or validation failure is returned
- **THEN** the error value SHALL expose its kind programmatically (variant + typed fields), not solely via message text

#### Scenario: Write to read-only shared region
- **WHEN** a WASM module attempts to write to a shared region page mapped with `PROT_READ`
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Read from mapped shared region
- **WHEN** a WASM module reads memory at an offset within a mapped shared region
- **THEN** the read SHALL succeed and return the shared memory contents

### Requirement: Table operations
The runtime SHALL support WebAssembly table operations including get, set, and size.

### Requirement: Cross-module import aliasing
The runtime SHALL preserve shared state for imported guest functions, tables, memories, and globals across module boundaries. Imported tables SHALL be shared by reference (mutations visible to all importers), and nested instantiation for imported guest functions SHALL share the caller's store (native registry and shared-memory registry).

#### Scenario: Imported table aliases exported table state
- **GIVEN** module A exports a table and module B imports that table
- **WHEN** module B mutates the imported table contents
- **THEN** subsequent reads through module A SHALL observe the same table contents

#### Scenario: Imported guest function binding executes real guest code
- **GIVEN** module A exports a WebAssembly function and module B imports it
- **WHEN** module B calls the imported function directly or through a funcref stored in a table
- **THEN** the exported WebAssembly function body from module A SHALL execute with the correct type checks and results, with access to the same store state as the caller

### Requirement: Global variables
The runtime SHALL support reading and writing mutable global variables.

### Requirement: Trap handling
The runtime SHALL propagate traps as errors and provide trap codes for common failure modes.

### Requirement: Callback-safe lock discipline
The engine SHALL NOT hold any store, instance, memory, or registry lock across a call into embedder-provided code (`HostFunc` implementations), and lock acquisition order across these objects SHALL follow a single global order to prevent ABBA deadlock.

#### Scenario: Host callback may re-enter engine APIs
- **WHEN** a `HostFunc` implementation calls back into engine APIs that acquire store or registry locks
- **THEN** the call SHALL complete without deadlock

#### Scenario: Concurrent attach and instance drop
- **WHEN** one thread attaches/detaches shared regions on a memory while another thread drops an instance sharing that memory and registry
- **THEN** both operations SHALL complete without deadlock and attachment accounting SHALL remain consistent

### Requirement: Error handling
The runtime SHALL use `Result<T, WasmError>` for all fallible operations with structured error types. `WasmError` SHALL use structured, typed variants (via `thiserror`) rather than free-form string payloads where variant data has known shape; variants constructed or matched by known embedders (`Runtime`, `Instantiate`) SHALL remain constructible/matchable with compatible shapes or be migrated with the embedder.

### Requirement: Thread safety
The runtime SHALL support `Send + Sync` on types where it is safe to share across threads.

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

### Requirement: Mmap-Backed Memory
Guest linear memory SHALL be backed by an `mmap`-based allocation rather than `Vec<u8>`, with a pre-reserved virtual address range supporting growth via `mprotect`.

#### Scenario: Memory created with mmap backing
- **WHEN** a `Memory` is created with `min_pages = 1`
- **THEN** the underlying storage SHALL be an `mmap`'d region with the full maximum virtual address range reserved

#### Scenario: Memory growth extends accessible range
- **WHEN** `Memory::grow(1)` is called
- **THEN** the additional pages SHALL be made accessible via `mprotect` without reallocation

### Requirement: Shared Page Tracking
The `Memory` struct SHALL track which page ranges are owned vs. mapped from shared regions, including the region identifier and protection level for each shared range.

#### Scenario: Shared range queryable
- **WHEN** a guest has attached shared regions
- **THEN** the `Memory` SHALL report the page offsets, region IDs, and protection levels of all attached regions

#### Scenario: Shared instance across threads
- **WHEN** an `Arc<Instance>` is created and shared between threads
- **THEN** compilation succeeds only if the instance is thread-safe
