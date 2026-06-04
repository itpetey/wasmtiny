## ADDED Requirements

### Requirement: Module initialization
The runtime SHALL provide a `Module` struct representing a loaded WASM module with types, functions, memories, tables, globals, and exports.

### Requirement: Instance creation
The runtime SHALL allow instantiation of a module into an `Instance` with isolated linear memory and table spaces.

### Requirement: Function invocation
The runtime SHALL support calling exported functions with typed arguments and return values via `Instance::call`.

### Requirement: Memory access
The runtime SHALL provide safe read/write access to linear memory with bounds checking. Memory access SHALL include both owned pages and mapped shared region pages. Writes to read-only shared pages SHALL trap before reaching memory.

#### Scenario: Out of bounds memory access
- **WHEN** a WASM module attempts to read memory at an offset beyond allocation
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Write to read-only shared region
- **WHEN** a WASM module attempts to write to a shared region page mapped with `PROT_READ`
- **THEN** a trap error is returned with `TrapCode::MemoryOutOfBounds`

#### Scenario: Read from mapped shared region
- **WHEN** a WASM module reads memory at an offset within a mapped shared region
- **THEN** the read SHALL succeed and return the shared memory contents

### Requirement: Table operations
The runtime SHALL support WebAssembly table operations including get, set, and size.

### Requirement: Cross-module import aliasing
The runtime SHALL preserve shared state for imported guest functions, tables, memories, and globals across module boundaries.

#### Scenario: Imported table aliases exported table state
- **GIVEN** module A exports a table and module B imports that table
- **WHEN** module B mutates the imported table contents
- **THEN** subsequent reads through module A SHALL observe the same table contents

#### Scenario: Imported guest function binding executes real guest code
- **GIVEN** module A exports a WebAssembly function and module B imports it
- **WHEN** module B calls the imported function directly or through a funcref stored in a table
- **THEN** the exported WebAssembly function body from module A SHALL execute with the correct type checks and results

### Requirement: Global variables
The runtime SHALL support reading and writing mutable global variables.

### Requirement: Trap handling
The runtime SHALL propagate traps as errors and provide trap codes for common failure modes.

### Requirement: Error handling
The runtime SHALL use `Result<T, WasmError>` for all fallible operations with structured error types.

### Requirement: Thread safety
The runtime SHALL support `Send + Sync` on types where it is safe to share across threads.

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

#### Scenario: Successful function call
- **WHEN** a valid module is instantiated and an exported function is called with correct arguments
- **THEN** the function executes and returns the expected result

#### Scenario: Type mismatch in function call
- **WHEN** a function is called with arguments of incorrect type
- **THEN** a validation error is returned

#### Scenario: Shared instance across threads
- **WHEN** an `Arc<Instance>` is created and shared between threads
- **THEN** compilation succeeds only if the instance is thread-safe
