## REMOVED Requirements

### Requirement: Cooperative Safepoints
**Reason**: The sole consumer implements cooperative re-entry (`__selium_guest_poll` + mailbox writes) instead of engine suspension, and never configures safepoints. The machinery (suspend.rs, interpreter safepoint checks) is removed.
**Migration**: Drive asynchronous completion through the host's call/poll protocol, as Selium does.

### Requirement: Resumable Execution State
**Reason**: Removed with the suspension machinery; mid-execution suspend/resume is no longer supported (the snapshot/restore subsystem that would persist such state is also removed).
**Migration**: Structure guest work as re-entrant poll invocations with state in linear memory.

### Requirement: Explicit Unsupported-State Failure
**Reason**: Removed with the suspension machinery; there are no suspension requests to reject.
**Migration**: No embedder action.
