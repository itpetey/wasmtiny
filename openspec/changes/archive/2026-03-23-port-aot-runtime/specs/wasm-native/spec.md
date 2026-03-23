## ADDED Requirements

### Requirement: Native Function Registration
The runtime SHALL allow registration of host functions that can be called from WASM.

#### Scenario: Register function with signature
- **WHEN** a native function is registered with a function type signature
- **THEN** the function SHALL be callable from WASM code via import section

### Requirement: Native Function Invocation
The runtime SHALL invoke native functions with correct parameter passing.

#### Scenario: Call native function with parameters
- **WHEN** WASM calls an imported native function with arguments
- **THEN** the native function SHALL receive the correct arguments

#### Scenario: Return values from native function
- **WHEN** a native function returns values
- **THEN** the values SHALL be pushed to the WASM stack

### Requirement: Native Function Table
The runtime SHALL support native function tables for indirect calls.

#### Scenario: Register native functions in table
- **WHEN** native functions are registered in a table
- **THEN** WASM can call them via call_indirect