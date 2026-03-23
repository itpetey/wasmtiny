## Context

The wasmtiny project is a Rust implementation of a WebAssembly runtime based on WAMR. The current documentation at the repository root describes the C/C++ WAMR implementation and is not applicable to the new Rust codebase. The source code is organized in src/ with modules for interpreter, jit, aot_runtime, loader, memory, and runtime.

## Goals / Non-Goals

**Goals:**
- Rewrite README.md to introduce wasmtiny as a Rust WebAssembly runtime
- Rewrite SUMMARY.md with appropriate navigation for Rust documentation
- Rewrite doc/ folder with Rust-specific content: building, embedding, API reference
- Rewrite gitbook/ folder with restructured documentation suitable for the Rust project

**Non-Goals:**
- Maintaining C/C++ WAMR documentation (handled elsewhere)
- Creating language bindings documentation for Python/Go (not part of this project)
- Writing detailed API reference for internal implementation details

## Decisions

- **Documentation structure**: Mirror standard Rust project layout with README, doc folder for guides, and gitbook for structured content
- **Content approach**: Focus on high-level usage and embedding the runtime rather than internal implementation details
- **Format**: Use Markdown for all documentation files

## Risks / Trade-offs

- [Risk] Documentation may become outdated as the API evolves → Mitigation: Keep documentation focused on stable public APIs
- [Risk] Large content rewrite may miss important details → Mitigation: Review each file against the source code modules