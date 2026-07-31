## ADDED Requirements

### Requirement: Binary format parsing
The loader SHALL parse WebAssembly binary format (.wasm) into an intermediate representation.

### Requirement: Validation
The loader SHALL validate WASM modules according to the WebAssembly specification, rejecting invalid modules.

### Requirement: Type checking
The loader SHALL verify that function signatures, local types, and global types are consistent throughout the module.

### Requirement: Section ordering
The loader SHALL enforce proper section ordering per the WASM binary specification for the sections it supports. Tag sections (exception handling) are not supported and SHALL be rejected.

#### Scenario: Unknown or unsupported section rejected
- **WHEN** a module contains a tag section or an unknown section id
- **THEN** parsing SHALL fail with an explicit unsupported-section error

### Requirement: Reference type encoding support
The loader SHALL parse and validate the basic reference-type encodings needed for funcref tables (`funcref`, `externref`, `ref.null`, `ref.func`, non-null table initialisers). Exception-handling tag encodings and GC heap types are not supported and SHALL be rejected explicitly.

#### Scenario: Non-null table initializer rejects ref.null
- **WHEN** a module declares a non-null table type but uses `ref.null` as the declared initializer
- **THEN** parsing or validation SHALL fail instead of accepting the initializer

#### Scenario: Tag imports and exports are rejected
- **WHEN** a module declares tag imports, tag exports, or a tag section
- **THEN** parsing or validation SHALL fail with an explicit unsupported-feature error

#### Scenario: GC heap types are rejected
- **WHEN** a module references GC heap types (e.g. `any`, `eq`, `struct`, `array` encodings)
- **THEN** parsing or validation SHALL fail with an explicit unsupported-feature error

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
