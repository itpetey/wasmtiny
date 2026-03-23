## ADDED Requirements

### Requirement: Cranelift integration
The JIT SHALL use Cranelift as the compiler backend for native code generation.

### Requirement: WASM-to-ISLE translation
The JIT SHALL translate WASM bytecode into Cranelift IR using WASM calling conventions.

### Requirement: Fast compilation
The JIT SHALL compile WASM functions faster than LLVM-based AOT compilation.

### Requirement: On-stack replacement
The JIT SHALL support OSR from interpreted code to JIT-compiled code.

### Requirement: Code caching
The JIT SHALL cache compiled code for reuse across instantiations of the same module.

### Requirement: Tiered compilation
The JIT SHALL support multiple compilation tiers with different optimization levels.

### Requirement: Trampoline generation
The JIT SHALL generate efficient entry trampolines for indirect calls through tables.

#### Scenario: JIT compile simple function
- **WHEN** a simple WASM function `i32.add` is JIT-compiled via Cranelift
- **THEN** native code is generated and executes correctly

#### Scenario: Code cache hit
- **WHEN** the same module is instantiated twice
- **THEN** the second instantiation reuses cached native code

#### Scenario: OSR from interpreter
- **WHEN** a hot loop is detected in interpreted code
- **THEN** the loop is OSR-compiled and execution switches to JIT code

#### Scenario: Indirect call through table
- **WHEN** a module performs an indirect call with table index 5
- **THEN** the JIT-generated code correctly dispatches to the target function
