## 1. Setup and Dependencies

- [x] 1.1 Add llvm-sys crate to Cargo.toml (feature-gated with "llvm-jit" feature)
- [x] 1.2 Add llvm-sys version constraints (LLVM 14+)
- [x] 1.3 Create src/jit/llvm_backend.rs module
- [x] 1.4 Create src/jit/wasm_to_llvm.rs module

## 2. LLVM ORC JIT Integration

- [x] 2.1 Initialize ORC JIT stack (ThreadSafeContext, ExecutionSession)
- [x] 2.2 Create JITDylib for module linking
- [x] 2.3 Implement symbol resolver for WASM imports
- [x] 2.4 Implement lazy compilation (compile on first call)
- [x] 2.5 Add proper memory management for compiled code

## 3. WASM to LLVM IR Translation

- [x] 3.1 Implement type mapping (WASM i32/i64/f32/f64 → LLVM types)
- [x] 3.2 Implement function translation (params, locals, body)
- [x] 3.3 Add local variable allocation (alloca in entry block)
- [x] 3.4 Implement i32 arithmetic operations translation
- [x] 3.5 Implement i64 arithmetic operations translation
- [x] 3.6 Implement f32/f64 floating point operations translation
- [x] 3.7 Implement memory load/store translation (call to helpers)
- [x] 3.8 Implement control flow translation (blocks, loops, branches)
- [x] 3.9 Implement function call translation (direct and indirect)
- [x] 3.10 Implement return value handling

## 4. LLVM Optimization Pipeline

- [x] 4.1 Configure optimization level (O2 for llvm-jit)
- [x] 4.2 Add standard optimization passes
- [x] 4.3 Configure target machine for x86-64
- [x] 4.4 Enable target-specific optimizations

## 5. Runtime Integration

- [x] 5.1 Add memory accessor helper functions (runtime bridge)
- [x] 5.2 Add trap handler integration
- [x] 5.3 Add host function call trampolines
- [x] 5.4 Integrate with WasmApplication for llvm-jit mode selection
- [x] 5.5 Add fallback to interpreter on compile failure

## 6. Testing

- [x] 6.1 Run existing interpreter/fast-jit tests (no regressions)
- [x] 6.2 Unpark and run llvm-jit regression tests from tests/regression.rs
- [x] 6.3 Add unit tests for WASM→LLVM IR translation
- [x] 6.4 Add integration tests for end-to-end llvm-jit execution

## 7. Cleanup

- [x] 7.1 Update src/lib.rs to export new LLVM JIT modules
- [x] 7.2 Run cargo fmt and cargo clippy
- [x] 7.3 Verify all llvm-jit regression tests pass

## Notes

- Fixed LLVM 17 opaque pointer crashes by replacing `LLVMGetElementType(LLVMTypeOf(...))` with APIs that operate on concrete value types: `LLVMGetAllocatedType` for allocas and `LLVMGlobalGetValueType` for function values.
