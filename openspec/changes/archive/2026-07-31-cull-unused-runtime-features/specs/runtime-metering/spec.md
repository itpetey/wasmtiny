## REMOVED Requirements

### Requirement: Instance Runtime Statistics
**Reason**: The sole consumer (Selium) computes metering host-side from its own `MeteringObservation` flow and never queries engine statistics. Engine metering cost two lock acquisitions per executed instruction on the hot path for no consumer.
**Migration**: Compute execution/resource statistics in the host, as Selium already does.

### Requirement: Monotonic Metering
**Reason**: Removed with the metering subsystem.
**Migration**: Host-side metering owns monotonicity guarantees.

### Requirement: Memory Usage Reporting
**Reason**: Removed with the metering subsystem; memory growth remains bounded by the module's declared maximum.
**Migration**: Track memory usage host-side via `Memory` size queries if needed.
