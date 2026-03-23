## ADDED Requirements

### Requirement: Cargo workspace structure
The system SHALL provide a Cargo workspace at `/examples/Cargo.toml` that includes all example crates as members.

#### Scenario: Workspace loads successfully
- **WHEN** user runs `cargo metadata` in `/examples/` directory
- **THEN** all example crates are listed as workspace members

#### Scenario: Individual crate builds
- **WHEN** user runs `cargo build -p <example-name>` in `/examples/`
- **THEN** the specified example crate compiles without errors

### Requirement: Example crate structure
Each example crate SHALL have a standard Rust structure with `src/main.rs` and `Cargo.toml`.

#### Scenario: Crate directory exists
- **WHEN** user lists `/examples/<name>/`
- **THEN** `Cargo.toml` and `src/main.rs` (or `lib.rs`) are present

#### Scenario: Crate compiles standalone
- **WHEN** user runs `cargo build` inside `/examples/<name>/`
- **THEN** the crate compiles to a binary or library without errors

### Requirement: WAMR FFI bindings
Example crates SHALL have access to WAMR runtime APIs through Rust FFI bindings.

#### Scenario: Bindings available
- **WHEN** example crate includes `wamr-sys` or generates bindings
- **THEN** WAMR functions (e.g., `wasm_runtime_init`, `wasm_runtime_load`) are callable from Rust

#### Scenario: Can instantiate runtime
- **WHEN** example calls WAMR initialization functions
- **THEN** runtime state is created and ready for module loading

### Requirement: Example coverage
The workspace SHALL include equivalent examples for all existing C samples.

#### Scenario: Basic example exists
- **WHEN** user lists `/examples/`
- **THEN** `basic` crate exists and demonstrates basic Wasm loading/execution

#### Scenario: Multi-thread example exists
- **WHEN** user lists `/examples/`
- **THEN** `multi-thread` crate exists and demonstrates thread creation

#### Scenario: WASI example exists
- **WHEN** user lists `/examples/`
- **THEN** `wasi-threads` crate exists and demonstrates WASI threading API

### Requirement: Example documentation
Each example crate SHALL include documentation explaining its purpose and usage.

#### Scenario: Crate docs build
- **WHEN** user runs `cargo doc` in an example crate
- **THEN** documentation is generated without warnings

#### Scenario: README present
- **WHEN** user reads `/examples/<name>/README.md`
- **THEN** usage instructions and description are provided