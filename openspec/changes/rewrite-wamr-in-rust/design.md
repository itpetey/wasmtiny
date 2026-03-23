## Context

The wasm-micro-runtime (WAMR) is a mature WebAssembly runtime written in C with approximately 500K+ lines of code across core/iwasm. The codebase includes:
- **Interpreter**: Classic and fast execution modes (`wasm_interp_classic.c`, `wasm_interp_fast.c`)
- **AOT Compiler**: LLVM-based ahead-of-time compilation (`aot_compiler.c`, `aot_llvm.c`)
- **AOT Runtime**: Execution engine for pre-compiled WASM (`aot_runtime.c`)
- **Fast JIT**: Experimental JIT compiler (`fast-jit/`)
- **Loader**: WASM module parsing and validation (`wasm_loader.c`)
- **GC**: Garbage collection support (`gc/`)
- **Memory**: Linear memory management (`wasm_memory.c`)

Current limitations in C:
- No memory safety guarantees, frequent buffer overflows and use-after-free bugs
- Manual memory management with error-prone allocation patterns
- Error handling via integer return codes instead of sum types
- Limited code reuse through inheritance patterns using function pointers
- No built-in testing framework

## Goals / Non-Goals

**Goals:**
- Rewrite core execution engine (interpreter, loader, runtime core) in idiomatic Rust
- Achieve memory safety with zero runtime overhead (no GC pauses)
- Replace error-code returns with `Result<T, Error>` for compile-time error handling
- Implement comprehensive unit tests alongside each component
- Maintain C API compatibility via unsafe FFI for existing consumers
- Leverage Rust's trait system for polymorphic behavior instead of C function pointers

**Non-Goals:**
- WASI support (explicitly excluded)
- Full LLVM AOT compilation (requires significant C++/LLVM binding work)
- Modifying language-bindings, product-mini, samples, tests, test-tools, wamr-sdk, zephyr, docs, gitbook, or ci
- Binary compatibility with existing WAMR builds
- Rewriting test-tools or test suites

## Decisions

### 1. Cargo Workspace Structure
**Decision**: Create `core/rust-wamr/` as a Rust crate with multiple modules.

```
core/rust-wamr/
├── Cargo.toml          # Workspace manifest
├── src/
│   ├── lib.rs          # Public API re-exports
│   ├── runtime/        # Core runtime engine
│   ├── interpreter/    # WASM interpreter
│   ├── loader/         # Module loading/validation
│   ├── aot_runtime/    # AOT execution
│   ├── memory/         # Linear memory management
│   ├── gc/             # Garbage collection
│   └── jit/            # Fast JIT (future)
└── c-api/              # Unsafe FFI bindings for C compatibility
```

**Rationale**: Single workspace allows shared dependencies and unified versioning while maintaining clear module boundaries.

**Alternatives considered**:
- Multiple crates per component: Too granular, complicates dependency management
- Flat structure: Loses organizational clarity

### 2. Memory Model
**Decision**: Use Rust's ownership model with `Box<T>` for heap allocation, `Arc<T>` for shared ownership, and `Vec<u8>` for byte buffers.

**Rationale**: Rust's borrow checker eliminates entire classes of memory bugs at compile time. `Box<T>` provides deterministic deallocation; `Arc<T>` enables safe sharing across threads.

**Alternatives considered**:
- Reference counting wrapper: Less efficient than Arc
- Custom allocator: Over-engineered for initial rewrite

### 3. Error Handling
**Decision**: Replace C error codes with `Result<T, WasmError>` using a structured error enum.

```rust
#[derive(Debug, Clone)]
pub enum WasmError {
    Validation(String),
    Load(String),
    Instantiate(String),
    Runtime(String),
    Trap(TrapCode),
}
```

**Rationale**: Forces error handling at compile time, provides rich error context, and integrates with `?` operator.

**Alternatives considered**:
- `anyhow::Error`: Too dynamic, loses type specificity
- Raw `i32` codes: Loses type safety

### 4. Trait-Based Polymorphism
**Decision**: Replace C function pointers with Rust traits.

```rust
pub trait Executable {
    fn execute(&self, ctx: &mut RuntimeContext) -> Result<(), WasmError>;
}
```

**Rationale**: Type-safe, composable, enables trait objects for dynamic dispatch when needed.

**Alternatives considered**:
- Enum with match arms: Inflexible for extensibility
- Type erasure with `Box<dyn Fn>`: Less performant

### 5. Module Representation
**Decision**: Represent WASM types with Rust structs and enums.

```rust
pub struct Module {
    pub types: Vec<FunctionType>,
    pub functions: Vec<Function>,
    pub memories: Vec<MemoryType>,
    pub tables: Vec<TableType>,
    pub globals: Vec<Global>,
    pub exports: Exports,
}
```

**Rationale**: Clear structure, enables pattern matching, integrates with serialization.

### 6. Interpreter Implementation
**Decision**: Implement stack-based interpreter using Rust vectors for operand and control stacks.

```rust
struct Interpreter {
    operand_stack: Vec<WasmValue>,
    control_stack: Vec<ControlFrame>,
}
```

**Rationale**: Idiomatic Rust, memory-safe, easy to test.

### 7. FFI Strategy
**Decision**: Expose C-compatible API in `c-api/` module using `#[repr(C)]` structs and `#[no_mangle] extern "C"` functions.

```rust
#[repr(C)]
pub struct wasm_module_t {
    // Opaque handle to Rust Module
    inner: *mut Module,
}
```

**Rationale**: Maintains backward compatibility while allowing internal Rust implementation.

### 8. JIT Strategy
**Decision**: Use Cranelift as the compiler backend for fast JIT compilation.

**Rationale**: Cranelift is battle-tested in production runtimes (Firefox, Wasmtime), supports multiple architectures, and handles complex WASM semantics correctly. Avoids reinventing register allocation and instruction selection.

**Alternatives considered**:
- Custom IR/codegen: Too much work, less tested
- LLVM: Heavyweight, complex FFI
- WASM-Micro-JIT (existing): Part of what we're replacing

### 9. Testing Strategy
**Decision**: Unit tests as `#[cfg(test)]` modules co-located with source files. Integration tests in `tests/` directory.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validation() {
        // ...
    }
}
```

**Rationale**: Tests live next to code they test, encourages testing during development.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Performance regression vs C | Profile critical paths, use `#[inline]`, unsafe for hot loops where safe |
| FFI overhead in C API | Minimize cross-boundary calls, batch operations |
| Rewrite scope creep | Strict adherence to non-goals, phased implementation |
| Breaking existing users | Maintain C API compatibility, version bump |
| Missing WASI features | Explicit non-goal, documented exclusion |

## Open Questions

1. **Garbage Collection**: WAMR's GC implementation is complex. Should we rewrite or wrap the existing C GC?
2. **Threading Model**: Should the Rust rewrite support WebAssembly threads proposal from the start?
3. **SIMD Support**: Should we use portable SIMD abstractions or target specific CPU features?
4. **Serialization**: Should we implement WASM binary format serialization in Rust or defer?
