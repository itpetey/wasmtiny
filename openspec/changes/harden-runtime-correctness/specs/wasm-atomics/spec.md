## ADDED Requirements

### Requirement: Spec-compliant atomic instruction encoding
The interpreter and validator SHALL decode 0xFE-prefixed atomic instructions using the subopcode assignments of the WebAssembly threads proposal (e.g. `memory.atomic.notify` = 0x00, `memory.atomic.wait32` = 0x01, `memory.atomic.wait64` = 0x02, `atomic.fence` = 0x03, loads at 0x10–0x16, stores at 0x17–0x1D, RMW ops at 0x1E–0x4E including cmpxchg at 0x48–0x4E).

#### Scenario: Spec-encoded module executes correctly
- **WHEN** a module assembled with standard toolchain encodings of atomic instructions (e.g. `i32.atomic.rmw.add` at subopcode 0x1E) is loaded and executed
- **THEN** the instructions SHALL decode to their spec-defined semantics and produce correct results

#### Scenario: Instruction-stream scanner stays synchronised
- **WHEN** an atomic instruction appears inside a `block`/`loop`/`if` body that the interpreter scans for control structure
- **THEN** the scanner SHALL skip the atomic subopcode and its immediates exactly, preserving correct control-flow resolution

### Requirement: Fallible atomic execution
Atomic instruction execution SHALL return errors/traps for all failure conditions (out-of-bounds address, misaligned address, stack/type faults) and SHALL never panic the host process, regardless of guest-supplied operands.

#### Scenario: Out-of-bounds atomic access traps
- **WHEN** a guest executes an atomic load/store/RMW at an out-of-bounds address
- **THEN** execution traps with `TrapCode::MemoryOutOfBounds` and the host process remains alive

#### Scenario: Misaligned atomic access traps
- **WHEN** a guest executes an atomic operation at an address not naturally aligned to the access size
- **THEN** execution traps with an alignment error

### Requirement: Validator-interpreter signature agreement
The validator's type signatures for atomic instructions SHALL exactly match the interpreter's execution semantics (operand counts, widths, and result types) for every supported subopcode.

#### Scenario: Validated atomic module executes without host panic
- **WHEN** any module passing validation contains i64 atomic operations, cmpxchg, or wait/notify
- **THEN** interpretation SHALL NOT produce stack type mismatches, underflows, or panics

## MODIFIED Requirements

### Requirement: Atomic load operations
The interpreter SHALL implement atomic load operations for i32, i64, i8, i16, and i32-wide (for i64) types. Narrow loads SHALL zero-extend and SHALL read exactly the encoded access width.

#### Scenario: i32.atomic.load returns value
- **GIVEN** shared memory with value 0x12345678 at address 0
- **WHEN** i32.atomic.load is executed
- **THEN** it SHALL return 0x12345678

#### Scenario: i64.atomic.load returns value
- **GIVEN** shared memory with value 0x123456789ABCDEF0 at address 0
- **WHEN** i64.atomic.load is executed
- **THEN** it SHALL return 0x123456789ABCDEF0

#### Scenario: i64.atomic.load8_u zero-extends
- **GIVEN** shared memory with byte 0x80 at address 0
- **WHEN** i64.atomic.load8_u is executed
- **THEN** it SHALL return 0x80 (128), not a sign-extended value
