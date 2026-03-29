## ADDED Requirements

### Requirement: Interpreter handles memory64 load instructions
The interpreter SHALL execute memory64 load instructions correctly.

#### Scenario: i64.load
- **GIVEN** a memory64 with value 0x12345678_9ABCDEF0 at address 8
- **WHEN** interpreter executes i64.load with offset 8
- **THEN** it SHALL return 0x12345678_9ABCDEF0

#### Scenario: i64.load8_s extends sign
- **GIVEN** a memory64 with byte 0xFF at address 0
- **WHEN** interpreter executes i64.load8_s
- **THEN** it SHALL return -1 (0xFFFFFFFFFFFFFFFF)

#### Scenario: i64.load8_u zero-extends
- **GIVEN** a memory64 with byte 0xFF at address 0
- **WHEN** interpreter executes i64.load8_u
- **THEN** it SHALL return 255

### Requirement: Interpreter handles memory64 store instructions
The interpreter SHALL execute memory64 store instructions correctly.

#### Scenario: i64.store
- **GIVEN** a memory64
- **WHEN** interpreter executes i64.store with value 0xDEADBEEF
- **THEN** subsequent i64.load SHALL return 0xDEADBEEF

#### Scenario: i64.store8 truncates
- **GIVEN** a memory64
- **WHEN** interpreter executes i64.store8 with value 0x1234
- **THEN** subsequent i64.load8_u SHALL return 0x34

### Requirement: Interpreter handles memory.size and memory.grow for memory64
The interpreter SHALL return i64 values for memory64 operations.

#### Scenario: memory.size returns i64
- **GIVEN** a memory64 with 256 pages
- **WHEN** interpreter executes memory.size
- **THEN** it SHALL return 256 as i64

#### Scenario: memory.grow accepts i64
- **GIVEN** a memory64 with 100 pages
- **WHEN** interpreter executes memory.grow with 50
- **THEN** it SHALL return 100 and new size SHALL be 150

### Requirement: Interpreter handles memory64 out-of-bounds
The interpreter SHALL trap on out-of-bounds memory64 accesses.

#### Scenario: Load beyond bounds
- **GIVEN** a memory64 with 10 pages (640 KiB)
- **WHEN** interpreter executes i64.load at offset 1_000_000
- **THEN** it SHALL trap with out-of-bounds error
