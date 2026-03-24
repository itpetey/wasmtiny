## ADDED Requirements

### Requirement: Fast JIT compiler compiles WASM to x64 machine code
The fast-jit compiler SHALL translate valid WASM bytecode modules into executable x64 machine code that can be directly executed by the CPU without interpretation.

#### Scenario: Compile simple add function
- **WHEN** a WASM module containing `(func (export "add") (param i32 i32) (result i32) (local.get 0) (local.get 1) (i32.add))` is loaded and compiled
- **THEN** the generated x64 code returns the sum of the two input parameters when called

#### Scenario: Compile memory load/store
- **WHEN** a WASM module with memory access `(i32.load (i32.const 0))` is compiled
- **THEN** the generated code reads from the WASM linear memory at the correct offset with bounds checking

#### Scenario: Fallback to interpreter for unimplemented opcodes
- **WHEN** compiling a function containing a WASM opcode not yet implemented in fast-jit (e.g., SIMD V128)
- **THEN** the compiler returns an error indicating fallback is required, allowing the interpreter to handle execution

### Requirement: Linear scan register allocation
The compiler SHALL use linear scan register allocation to efficiently assign x64 registers to WASM locals and stack values.

#### Scenario: Allocate registers for multiple values
- **WHEN** compiling a function with 4 i32 parameters and multiple local variables
- **THEN** the allocator assigns available registers (RAX, RBX, RCX, RDX, R8, R9) to values, spilling to stack only when all registers are in use

#### Scenario: Handle register pressure
- **WHEN** compiling a function with more live values than available registers
- **THEN** the allocator spills least-recently-used values to the stack, reloading them when needed

### Requirement: Instruction emission for common WASM ops
The compiler SHALL emit x64 instructions for the core WASM instruction set covering arithmetic, logical, memory, and control flow operations.

#### Scenario: Emit i32 arithmetic
- **WHEN** compiling WASM i32.add, i32.sub, i32.mul, i32.div_s, i32.div_u, i32.rem_s, i32.rem_u
- **THEN** appropriate x64 instructions (ADD, SUB, IMUL, IDIV, DIV, etc.) are emitted with correct operand sizing

#### Scenario: Emit i64 arithmetic
- **WHEN** compiling WASM i64.add, i64.sub, i64.mul, i64.div_s, i64.div_u
- **THEN** x64 instructions operating on 64-bit registers (RAX, RDX for division) are emitted

#### Scenario: Emit memory access with bounds check
- **WHEN** compiling i32.load or i32.store with offset and alignment
- **THEN** bounds check is performed before memory access, jumping to trap handler on out-of-bounds

#### Scenario: Emit control flow
- **WHEN** compiling block, loop, if, br, br_if, br_table, end, return
- **THEN** correct x64 jump instructions (JMP, JE, JNE, etc.) and labels are generated

### Requirement: Function call handling
The compiler SHALL emit efficient calling conventions for WASM→WASM calls and host function calls.

#### Scenario: Direct WASM function call
- **WHEN** compiling a call instruction to another function in the same module
- **THEN** a direct x64 jump/call to the compiled target function is emitted

#### Scenario: Host function call
- **WHEN** compiling a call to an imported host function
- **THEN** a call to a runtime trampoline is emitted that handles the transition from JIT code to host