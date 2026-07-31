## ADDED Requirements

### Requirement: Untrusted operand hardening
The interpreter SHALL NOT size heap allocations directly from guest-controlled counts or lengths, and SHALL bounds-check memory regions before copying or filling them.

#### Scenario: memory.copy bounds-checked before copying
- **WHEN** a guest executes `memory.copy` with a length that exceeds source or destination bounds
- **THEN** execution SHALL trap with `MemoryOutOfBounds` without allocating a length-sized staging buffer

#### Scenario: memory.fill bounds-checked before filling
- **WHEN** a guest executes `memory.fill` with a length that exceeds destination bounds
- **THEN** execution SHALL trap with `MemoryOutOfBounds` without allocating a length-sized staging buffer

#### Scenario: br_table with huge count does not exhaust memory
- **WHEN** a module executes a `br_table` whose declared label count is near u32::MAX
- **THEN** the interpreter SHALL process the instruction without pre-allocating count-sized memory (validation already bounded the count)

### Requirement: Robust immediate decoding
The interpreter SHALL reject LEB128 immediates whose final byte carries bits beyond the decoded type's width, and SHALL reject unmapped atomic subopcodes with an error rather than ignoring them.

#### Scenario: Overlong LEB immediate rejected
- **WHEN** a function body contains a u32 immediate encoded with set bits beyond bit 31
- **THEN** loading or execution SHALL fail with an explicit error

#### Scenario: Unknown atomic subopcode errors
- **WHEN** execution encounters an unmapped 0xFE subopcode
- **THEN** execution SHALL fail with an explicit unsupported-instruction error rather than a no-op

## MODIFIED Requirements

### Requirement: Stack overflow detection
The interpreter SHALL detect and trap on operand stack overflow. The interpreter's stack/call-depth limits SHALL be consistent with the validator's static guarantees, so that no module passing validation fails at runtime for exceeding a limit the validator did not check.

#### Scenario: Validator and interpreter limits agree
- **WHEN** a module's maximum operand-stack depth exceeds the interpreter's operand stack capacity
- **THEN** the module SHALL be rejected at validation time with a clear error rather than failing mid-execution
