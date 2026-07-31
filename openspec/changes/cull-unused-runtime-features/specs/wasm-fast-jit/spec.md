## REMOVED Requirements

### Requirement: Cranelift integration
**Reason**: The fast-JIT was 4,142 lines of unconditionally compiled scaffolding (an x86-64 byte emitter with no Cranelift) referenced by nothing and incapable of executing code. The sole consumer runs interpreter-only.
**Migration**: All execution uses the classic interpreter.

### Requirement: WASM-to-ISLE translation
**Reason**: Removed with the fast-JIT; no translation layer exists.
**Migration**: All execution uses the classic interpreter.

### Requirement: Fast compilation
**Reason**: Removed with the fast-JIT; there is no compilation tier.
**Migration**: All execution uses the classic interpreter.

### Requirement: On-stack replacement
**Reason**: Removed with the fast-JIT; no tier-up mechanism exists.
**Migration**: All execution uses the classic interpreter.

### Requirement: Code caching
**Reason**: Removed with the fast-JIT; no compiled code exists to cache.
**Migration**: All execution uses the classic interpreter.

### Requirement: Tiered compilation
**Reason**: Removed with the fast-JIT; the interpreter is the single execution tier.
**Migration**: All execution uses the classic interpreter.

### Requirement: Trampoline generation
**Reason**: Removed with the fast-JIT; indirect calls dispatch within the interpreter.
**Migration**: All execution uses the classic interpreter.
