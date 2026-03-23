## Context

The Rust implementation already has partial AOT loader and runtime. We need to extend it to fully replace the C implementation and add the application entry point and native function handling.

## Goals / Non-Goals

**Goals:**
- Complete AOT runtime with memory management, table handling, global access
- Port wasm_application.c for WASM application loading/execution
- Port wasm_native.c for native (host) function registration
- No GC - simple reference type handling without automatic collection

**Non-Goals:**
- LLVM-based AOT compilation (requires C++/LLVM bindings)
- GC support (reference types with manual management)
- Debug support (can be added later)

## Decisions

### 1. Application Entry Point
**Decision**: Create separate wasm-application module

Rationale: Keeps concerns separated - application loading is distinct from runtime core

### 2. Native Function Handling
**Decision**: Extend existing HostFunc trait in src/runtime/instance.rs

Rationale: Already defined, just needs full implementation

### 3. AOT Format
**Decision**: Keep existing binary format (don't change AOT file structure)

Rationale: Maintain compatibility with existing AOT files

## Risks / Trade-offs

- [Risk] AOT format complexity → [Mitigation] Focus on runtime only, not LLVM compiler
- [Risk] Integration testing → [Mitigation] Use existing wasm files for validation