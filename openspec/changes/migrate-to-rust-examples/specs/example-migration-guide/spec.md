## ADDED Requirements

### Requirement: Migration guide document
The project SHALL provide a guide for migrating C-based WAMR examples to Rust.

#### Scenario: Guide exists
- **WHEN** user reads `/examples/MIGRATION.md`
- **THEN** step-by-step instructions for C to Rust migration are present

#### Scenario: Covers key patterns
- **WHEN** user follows the guide
- **THEN** the document covers: FFI usage, build system setup, and common patterns

### Requirement: Example mapping documentation
The system SHALL document the mapping between original C examples and new Rust crates.

#### Scenario: Mapping table present
- **WHEN** user reads the migration guide
- **THEN** a table showing `samples/<name>` → `examples/<name>` mapping is included

### Requirement: Deprecation notice
The original C example directories SHALL be marked as deprecated.

#### Scenario: Deprecation notice in old location
- **WHEN** user reads `/samples/README.md`
- **THEN** a deprecation notice directing users to `/examples/` is present

#### Scenario: Deprecation notice in product-mini
- **WHEN** user reads `/product-mini/app-samples/README.md`
- **THEN** a deprecation notice directing users to `/examples/` is present