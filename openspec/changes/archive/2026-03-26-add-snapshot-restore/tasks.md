## 1. Snapshot State Model

- [x] 1.1 Define the canonical snapshot payload for instance state and compatibility metadata.
- [x] 1.2 Define runtime APIs for capture, restore, and resume.

## 2. Capture and Restore

- [x] 2.1 Implement safepoint-based snapshot capture for interpreter-managed state.
- [x] 2.2 Implement snapshot restore into a fresh runtime instance and rebuild any non-canonical execution artefacts.
- [x] 2.3 Add explicit errors for unsupported resources and incompatible restore targets.

## 3. Validation

- [x] 3.1 Add tests for capture/restore round-trips across memory, globals, tables, and resumed execution.
- [x] 3.2 Add tests for compatibility checks and unsupported-resource failures.
