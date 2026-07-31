## REMOVED Requirements

### Requirement: Configurable Instance Limits
**Reason**: The sole consumer never configures engine-level instance limits; resource policy lives in the host. The limits machinery was intertwined with the removed metering subsystem.
**Migration**: Enforce resource budgets host-side; memory growth remains bounded by the module's declared maximum.

### Requirement: Memory Limit Enforcement
**Reason**: Removed with instance limits; spec-defined memory maximums still bound growth.
**Migration**: Rely on the module's declared memory maximum, or enforce budgets host-side.

### Requirement: Execution Budget Enforcement
**Reason**: Removed with instance limits; the consumer bounds guest work by design of its hostcall/poll protocol.
**Migration**: Bound execution host-side (e.g. Selium's cooperative poll model).
