## ADDED Requirements

### Requirement: Binary format parsing
The loader SHALL parse WebAssembly binary format (.wasm) into an intermediate representation.

### Requirement: Validation
The loader SHALL validate WASM modules according to the WebAssembly specification, rejecting invalid modules.

### Requirement: Type checking
The loader SHALL verify that function signatures, local types, and global types are consistent throughout the module.

### Requirement: Section ordering
The loader SHALL enforce proper section ordering per the WASM binary specification.

#### Scenario: Tag section ordering is accepted
- **WHEN** a module places the tag section after the function section and before globals or exports, as allowed by the binary format
- **THEN** parsing and validation SHALL succeed

### Requirement: Name section support
The loader SHALL parse the custom name section and make it available for debugging.

### Requirement: Streaming parse
The loader SHALL support streaming/partial parsing for large modules.

### Requirement: Incremental validation
The loader SHALL provide incremental validation to detect errors early during loading.

### Requirement: Reference type encoding support
The loader SHALL parse and validate supported typed-reference and tag encodings used by modules under test.

#### Scenario: Non-null table initializer rejects ref.null
- **WHEN** a module declares a non-null table type but uses `ref.null` as the declared initializer
- **THEN** parsing or validation SHALL fail instead of accepting the initializer

#### Scenario: Tag imports and exports are accepted
- **WHEN** a module declares tag imports or tag exports with valid type indices
- **THEN** parsing and validation SHALL succeed

#### Scenario: Valid WASM module loading
- **WHEN** a valid WASM binary is loaded
- **THEN** a `Module` is returned with all sections parsed correctly

#### Scenario: Invalid magic number
- **WHEN** a file with invalid WASM magic bytes is loaded
- **THEN** `Err(WasmError::Load("invalid magic number"))` is returned

#### Scenario: Type mismatch in function body
- **WHEN** a function body references local variables of wrong type
- **THEN** `Err(WasmError::Validation("type mismatch"))` is returned

#### Scenario: Missing required section
- **WHEN** a module is missing the Type section
- **THEN** `Err(WasmError::Validation("type section required"))` is returned

#### Scenario: Large module streaming
- **WHEN** a large WASM module is loaded in chunks
- **THEN** parsing succeeds with valid intermediate state
