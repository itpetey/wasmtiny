## ADDED Requirements

### Requirement: Safepoint-Based Snapshot Capture
The runtime SHALL capture instance snapshots only from execution states that have been suspended at a runtime-approved safepoint.

#### Scenario: Capture suspended instance
- **WHEN** a caller requests a snapshot of an instance that is suspended at a valid safepoint
- **THEN** the runtime SHALL return a snapshot containing the canonical guest state needed for restore

### Requirement: Restore Compatible Snapshot
The runtime SHALL restore a compatible snapshot into a new instance and allow execution to resume from the captured semantic state.

#### Scenario: Restore and resume
- **WHEN** a caller restores a snapshot into a compatible runtime target
- **THEN** the runtime SHALL recreate the guest-visible state and allow execution to continue from the captured suspension point

### Requirement: Snapshot Compatibility Validation
The runtime SHALL validate snapshot compatibility before restore.

#### Scenario: Incompatible restore target
- **WHEN** a caller attempts to restore a snapshot into an incompatible runtime or module target
- **THEN** the runtime SHALL reject the restore with an explicit compatibility error

### Requirement: Explicit Unsupported-State Failure
The runtime SHALL fail snapshot capture when required state cannot be serialized safely.

#### Scenario: Unsupported runtime attachment
- **WHEN** a snapshot request encounters runtime state that is outside the supported snapshot contract
- **THEN** the runtime SHALL fail the capture explicitly instead of producing a partial snapshot
