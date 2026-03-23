## Why

The project now includes a native Rust implementation (wasmtiny) of a WebAssembly runtime based on WAMR. The existing documentation (doc/, gitbook/, README.md, SUMMARY.md) describes the C/C++ WAMR implementation and is not applicable to the Rust codebase. The documentation must be rewritten to reflect the Rust project structure, APIs, and usage patterns.

## What Changes

- Rewrite README.md to describe the Rust WebAssembly runtime, its features, and how to use it
- Rewrite SUMMARY.md to reflect the new documentation structure for the Rust project
- Rewrite doc/ folder with Rust-specific documentation covering:
  - Building and installation
  - Embedding the runtime
  - API reference
  - Examples
- Rewrite gitbook/ folder with restructured content appropriate for the Rust project

## Capabilities

### New Capabilities

- `rust-runtime-docs`: Complete documentation for the wasmtiny Rust WebAssembly runtime including API docs, embedding guide, and examples

### Modified Capabilities

- (none - new documentation for new Rust project)

## Impact

- README.md - main entry point for developers
- SUMMARY.md - navigation structure for documentation
- doc/ - technical documentation folder
- gitbook/ - GitBook formatted documentation