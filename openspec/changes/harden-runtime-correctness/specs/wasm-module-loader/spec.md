## ADDED Requirements

### Requirement: DataCount section support
The loader SHALL accept the DataCount section (id 12, ordered between the element and code sections) and use it to validate data segment indices referenced by `memory.init`/`data.drop`.

#### Scenario: Bulk-memory binary with DataCount loads
- **WHEN** a module produced by a standard toolchain (LLVM/Binaryen) containing `memory.init` or `data.drop` and a DataCount section is loaded
- **THEN** parsing and validation SHALL succeed

#### Scenario: Data segment index checked against DataCount
- **WHEN** a function body references a data segment index ≥ the DataCount value
- **THEN** validation SHALL fail

### Requirement: Untrusted declaration limits
The loader SHALL NOT size heap allocations directly from untrusted section counts or declared minimums, and SHALL cap function-local counts and table minimums at defined maxima.

#### Scenario: Huge type-section count does not pre-allocate
- **WHEN** a malformed module declares a section vector count near u32::MAX
- **THEN** parsing SHALL fail with an unexpected-end or count error without attempting a count-sized allocation

#### Scenario: Huge locals count rejected
- **WHEN** a function body declares locals whose expanded total exceeds the implementation cap
- **THEN** validation SHALL fail with an explicit limits error

#### Scenario: Huge table minimum rejected
- **WHEN** a table declaration's minimum exceeds the implementation cap
- **THEN** validation SHALL fail with an explicit limits error rather than allocating minimum-sized storage

### Requirement: Name encoding strictness
The loader SHALL require names (module, function, import/export fields) to be valid UTF-8 and reject malformed encodings.

#### Scenario: Invalid UTF-8 name rejected
- **WHEN** a module contains an import/export name that is not valid UTF-8
- **THEN** parsing SHALL fail with an explicit encoding error

## MODIFIED Requirements

### Requirement: Validation
The loader SHALL validate WASM modules according to the WebAssembly specification, rejecting invalid modules. This SHALL include rejecting an `if` without `else` whose parameter and result arities differ, rejecting non-zero memory indices on bulk-memory instructions (multi-memory is unsupported), validating memarg alignment exponents, and enforcing the `ref.func` declared-function rule.

#### Scenario: if without else and mismatched arities rejected
- **WHEN** a function contains an `if` block whose parameter count differs from its result count and which has no `else` arm
- **THEN** validation SHALL fail

#### Scenario: Bulk op with non-zero memory index rejected
- **WHEN** a function body contains `memory.copy`, `memory.fill`, or `memory.init` with a non-zero memory index immediate
- **THEN** validation SHALL fail

#### Scenario: Excessive memarg alignment rejected
- **WHEN** a memory instruction's alignment immediate encodes an alignment greater than its natural access size
- **THEN** validation SHALL fail
