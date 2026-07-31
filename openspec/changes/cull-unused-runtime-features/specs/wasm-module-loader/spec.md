## REMOVED Requirements

### Requirement: Name section support
**Reason**: The parsed representation was never populated (no writer), and debugging support is not a consumer requirement.
**Migration**: Custom name sections are skipped as opaque custom sections; diagnostics use function indices.

### Requirement: Streaming parse
**Reason**: The streaming loader re-parsed the entire accumulated buffer per chunk (O(n²)), detected incomplete input by string-matching error text, and had no consumer. Modules are loaded from complete in-memory byte slices.
**Migration**: Load modules from a complete `&[u8]` via the standard parse path.

### Requirement: Incremental validation
**Reason**: Tied to the removed streaming parse path; whole-module validation runs once after parsing.
**Migration**: Validation errors are reported from the single whole-module validation pass.

## MODIFIED Requirements

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
