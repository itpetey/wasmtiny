## 1. x64 Instruction Emitter

- [x] 1.1 Create src/jit/emitter.rs with x64 instruction encoding functions
- [x] 1.2 Implement emit_add, emit_sub, emit_mul for i32/i64
- [x] 1.3 Implement emit_div, emit_rem for i32/i64 (signed/unsigned)
- [x] 1.4 Implement emit_load (MOV with address computation)
- [x] 1.5 Implement emit_store (MOV with address computation)
- [x] 1.6 Implement emit_jmp, emit_jcc (conditional jumps)
- [x] 1.7 Implement emit_call, emit_ret
- [x] 1.8 Implement emit_comparison operations (CMP, TEST)

## 2. Register Allocator

- [x] 2.1 Create src/jit/regalloc.rs with linear scan allocator
- [x] 2.2 Implement register tracking (which registers are allocated)
- [x] 2.3 Implement spill slot allocation
- [x] 2.4 Implement register assignment for values
- [x] 2.5 Handle register conflicts and spills
- [x] 2.6 Add spill/reload instruction emission for spilled values

## 3. Fast JIT Compiler

- [x] 3.1 Refactor src/jit/compiler.rs to remove old IR translation stub
- [x] 3.2 Implement WASM to x64 compilation pass
- [x] 3.3 Add i32 arithmetic opcodes (add, sub, mul, div_s, div_u, rem_s, rem_u)
- [x] 3.4 Add i64 arithmetic opcodes
- [x] 3.5 Add i32/i64 comparison opcodes (eq, ne, lt_s, lt_u, gt_s, gt_u, etc.)
- [x] 3.6 Add logical opcodes (and, or, xor, shift, rotate)
- [x] 3.7 Add memory load/store opcodes with bounds checking
- [x] 3.8 Add control flow opcodes (block, loop, if, br, br_if, br_table)
- [x] 3.9 Add function call and return opcodes
- [x] 3.10 Add local get/set/tee operations

## 4. JIT Runtime Integration

- [x] 4.1 Extend src/jit/runtime.rs for compiled code execution
- [x] 4.2 Add code cache integration (store compiled functions)
- [x] 4.3 Implement memory accessor helpers for JIT code
- [x] 4.4 Add trap handler integration (bounds check failures, unreachable)
- [x] 4.5 Add host function call trampolines
- [x] 4.6 Integrate JIT compiler with WasmApplication

## 5. OSR Infrastructure (Tiered Compilation)

- [x] 5.1 Add call count tracking per function
- [x] 5.2 Implement OSR trigger detection (call count threshold)
- [x] 5.3 Add OSR entry point generation in compiled code
- [x] 5.4 Implement state transfer between tiers (stack values, locals)
- [x] 5.5 Add recompilation pipeline for optimized tier

## 6. Interpreter Fallback

- [x] 6.1 Add fallback detection for unsupported opcodes
- [x] 6.2 Implement graceful fallback to interpreter
- [x] 6.3 Add mixed execution mode (some functions JIT, some interpreted)

## 7. Testing

- [x] 7.1 Run existing interpreter tests to ensure no regressions
- [x] 7.2 Unpark and run fast-jit regression tests from tests/regression.rs
- [x] 7.3 Add unit tests for x64 emitter (verify instruction encoding)
- [x] 7.4 Add unit tests for register allocator
- [x] 7.5 Add integration tests for JIT execution

## 8. Cleanup

- [x] 8.1 Remove old stub IR code from compiler.rs
- [x] 8.2 Update src/lib.rs to export new JIT modules
- [x] 8.3 Run cargo fmt and cargo clippy
- [x] 8.4 Verify all regression tests pass