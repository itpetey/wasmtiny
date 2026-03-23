## Context

The WAMR project currently has example applications in:
- `product-mini/app-samples/` - Platform-specific sample applications
- `samples/` - General purpose samples (27 directories including basic, multi-thread, wasi-threads, etc.)

These are built using C with CMake. There's no unified Rust-based example system.

## Goals / Non-Goals

**Goals:**
- Create a unified `/examples/` directory at project root with Rust crate workspace
- Migrate all existing C examples to Rust equivalents
- Provide clear documentation for each example
- Maintain feature parity with existing C examples

**Non-Goals:**
- Modify the core WAMR C runtime (that remains in C)
- Migrate platform-specific build systems (Android, iOS, etc.)
- Create a full Rust SDK - examples use FFI bindings to C core

## Decisions

1. **Cargo Workspace Structure**:
   - Root `examples/Cargo.toml` defines workspace
   - Each example is a crate in `examples/<name>/`
   - Decision: Flat structure (no nested directories) for simplicity

2. **FFI to C Core**:
   - Use `wasm-runtime-common` C library via `bindgen`
   - Decision: Generate bindings at build time using `cbindgen` or `bindgen`

3. **Example Organization**:
   - Map existing directories: `samples/basic` → `examples/basic`
   - Map: `product-mini/app-samples/` → `examples/product-mini`
   - Decision: Keep original names where possible for familiarity

4. **Build Approach**:
   - Each example builds WAMR static library first, then links
   - Use `build.rs` for C component compilation
   - Decision: Inline C compilation in each crate (not separate workspace crate)

5. **Testing**:
   - Each example includes a simple test that runs the Wasm module
   - Decision: No integration tests across examples (keep isolated)

## Risks / Trade-offs

- **[Risk] Build complexity**: Compiling C core in each crate is repetitive
  - → [Mitigation] Consider a shared `wamr-sys` crate for bindings

- **[Risk] Maintenance burden**: Two codebases to maintain during migration
  - → [Mitigation] Deprecate C examples immediately, remove after migration period

- **[Risk] Platform support**: Some examples are platform-specific (SGX, WASI threads)
  - → [Mitigation] Add platform-gated examples with `cfg` attributes

- **[Trade-off] Performance**: Rust FFI adds overhead vs direct C
  - [Mitigation] Accept for example code clarity; core remains C