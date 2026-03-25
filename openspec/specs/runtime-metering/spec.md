## ADDED Requirements

### Requirement: Instance Runtime Statistics
The runtime SHALL expose per-instance metering data for guest execution and resource usage.

#### Scenario: Query instance stats
- **WHEN** a caller requests runtime statistics for an instance
- **THEN** the runtime SHALL return the current metering data for that instance

### Requirement: Monotonic Metering
The runtime SHALL report execution counters that do not move backwards during an instance lifetime.

#### Scenario: Observe repeated samples
- **WHEN** a caller samples runtime metering data multiple times while an instance executes
- **THEN** the reported execution counters SHALL remain monotonic for that instance

### Requirement: Memory Usage Reporting
The runtime SHALL report current instance memory usage in a way that higher layers can inspect.

#### Scenario: Report memory usage
- **WHEN** an instance allocates or grows memory
- **THEN** the runtime SHALL make the resulting memory usage observable through the metering API
