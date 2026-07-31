## REMOVED Requirements

### Requirement: JIT compiler emits atomic operations
**Reason**: There is no JIT compiler; atomic operations are interpreted (see `wasm-atomics`).
**Migration**: Atomic instructions execute in the interpreter against shared linear memory.
