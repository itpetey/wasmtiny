## ADDED Requirements

### Requirement: AOT module loading
The AOT runtime SHALL load pre-compiled AOT binaries into executable form.

### Requirement: Native function execution
The AOT runtime SHALL execute native-compiled WASM functions without interpretation.

### Requirement: Call frame management
The AOT runtime SHALL maintain WebAssembly stack frames with proper spill and restore of callee-saved registers.

### Requirement: Memory management
The AOT runtime SHALL provide memory allocation and deallocation for WASM linear memory.

### Requirement: Table management
The AOT runtime SHALL manage WebAssembly tables with type-safe element access.

### Requirement: Global variable access
The AOT runtime SHALL provide efficient access to mutable and immutable global variables.

### Requirement: Intrinsic function support
The AOT runtime SHALL implement WebAssembly intrinsic operations directly in native code.

#### Scenario: Load and execute AOT module
- **WHEN** a valid AOT binary is loaded and an exported function is called
- **THEN** the function executes at native speed

#### Scenario: AOT memory allocation
- **WHEN** a WASM module allocates memory via `memory.grow`
- **THEN** native memory is allocated and the previous size is returned

#### Scenario: AOT table element access
- **WHEN** a module calls a function through a table element
- **THEN** the correct function is invoked with proper type checking

#### Scenario: AOT global read
- **WHEN** a module reads a mutable global variable
- **THEN** the current value is returned with proper type conversion
