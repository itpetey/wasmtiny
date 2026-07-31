## REMOVED Requirements

### Requirement: Fast interpreter execution
**Reason**: The register-based "fast" interpreter was an unreferenced, broken prototype (incorrect LEB decoding, stubbed call/global semantics) kept compiling only via `#[allow(dead_code)]`. One interpreter execution mode — the classic stack-based engine — is sufficient for the sole consumer.
**Migration**: All execution uses the classic stack-based interpreter; there is no alternate execution mode to select.

## MODIFIED Requirements

### Requirement: Classic interpreter execution
The interpreter SHALL execute WebAssembly bytecode using a stack-based virtual machine with operand and control stacks. It SHALL be the only execution mode; it SHALL NOT contain safepoint, suspension, or per-instruction metering hooks (those subsystems are removed), and it SHALL NOT dispatch host calls through a pending/outcome protocol — host functions return results or errors synchronously.

#### Scenario: Host function call completes synchronously
- **WHEN** a guest calls an imported host function
- **THEN** the host function's `call` method runs to completion and its results or error are delivered directly to the interpreter
