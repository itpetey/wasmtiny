## REMOVED Requirements

### Requirement: AOT compiler handles atomic operations
**Reason**: There is no AOT compiler; atomic operations are interpreted (see `wasm-atomics`).
**Migration**: Atomic instructions execute in the interpreter against shared linear memory.
