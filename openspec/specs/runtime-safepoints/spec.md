## ADDED Requirements

### Requirement: Cooperative Safepoints
The runtime SHALL allow a running instance to suspend only at explicit safepoints where guest-visible execution state is consistent.

#### Scenario: Suspend at a host boundary
- **WHEN** an instance reaches a designated suspension boundary such as a hostcall or explicit yield point
- **THEN** the runtime SHALL allow the instance to suspend without losing guest-visible execution state

### Requirement: Resumable Execution State
The runtime SHALL preserve enough guest-visible state to resume a suspended instance from the same semantic execution point.

#### Scenario: Resume after suspension
- **WHEN** a previously suspended instance is resumed
- **THEN** the runtime SHALL continue execution with the same program counter, locals, operand stack, and memory-visible state that existed at suspension time

### Requirement: Explicit Unsupported-State Failure
The runtime SHALL reject suspension attempts that cannot be represented safely by the current execution model.

#### Scenario: Unsupported suspend request
- **WHEN** suspension is requested from a runtime state that is not safely resumable
- **THEN** the runtime SHALL return an explicit error instead of silently blocking or continuing with undefined behaviour
