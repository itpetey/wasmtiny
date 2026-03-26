## 1. Shared Region Model

- [x] 1.1 Define shared-region runtime objects, identifiers, and lifecycle APIs.
- [x] 1.2 Define attach/detach behaviour and mapping metadata for guest instances.

## 2. Runtime Integration

- [x] 2.1 Implement shared-region allocation and bounds-checked attachment in the interpreter path.
- [x] 2.2 Add JIT-aware shared-memory access plumbing that honours the same attachment semantics.
- [x] 2.3 Add failure handling for invalid mappings, detached regions, and unsupported attachment states.

## 3. Validation

- [x] 3.1 Add tests for multi-instance visibility and zero-copy access semantics.
- [x] 3.2 Add tests for detach, lifetime, and invalid-access behaviour.
