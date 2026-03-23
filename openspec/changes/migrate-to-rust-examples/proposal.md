## Why

The WebAssembly Micro Runtime (WAMR) project currently has example applications in two locations: `product-mini/app-samples/` and `samples/`. These are written in C with CMake build systems. Migrating these to Rust-based example crates in a unified `/examples/` directory would modernize the codebase, improve type safety, and make examples more accessible to Rust developers. This aligns with the trend of Rust becoming the preferred language for Wasm tooling.

## What Changes

- Create a new `/examples/` directory at the project root
- Convert existing `samples/` and `product-mini/app-samples/` examples to Rust crates
- Create a Cargo workspace at `/examples/` with individual crate directories
- Each example crate includes its source code, `Cargo.toml`, and build instructions
- Remove or deprecate the old C-based example locations after migration
- **BREAKING**: Old C-based build systems (CMake) for examples will be replaced

## Capabilities

### New Capabilities
- `rust-examples`: A workspace of Rust crate examples demonstrating WAMR APIs and features
- `example-migration-guide`: Documentation for migrating existing C examples to Rust

### Modified Capabilities
- None - this is a new capability, not modifying existing specs

## Impact

- New directory: `/examples/` containing Rust crate workspace
- Affected files: `samples/`, `product-mini/app-samples/` (to be migrated or deprecated)
- Build system: Shift from CMake to Cargo for examples
- Documentation: Update README and build instructions