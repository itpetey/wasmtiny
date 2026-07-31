## ADDED Requirements

### Requirement: Overflow-safe region arithmetic
All region size computations (alignment round-up, capacity checks) SHALL use checked or sufficiently wide arithmetic, and region sizes SHALL be capped at a defined maximum, so that no host-supplied size can overflow, panic in debug builds, or wrap to an undersized allocation in release builds.

#### Scenario: Oversized region request fails cleanly
- **WHEN** a caller requests a shared region whose size is near or above u32::MAX
- **THEN** allocation SHALL return an explicit error without panic, wrap, or undersized mapping

### Requirement: Host I/O bounds check integrity
Host-side region read/write operations SHALL validate `offset` and `length` with overflow-proof comparisons before any pointer arithmetic or `unsafe` copy.

#### Scenario: Overflowing offset rejected
- **WHEN** a host caller reads or writes a region at an offset whose `offset + length` would overflow `usize`
- **THEN** the operation SHALL return an explicit bounds error and perform no memory access

### Requirement: Unique region naming
Shared region backing-object names SHALL be unique across concurrent processes (incorporating process identity and/or entropy), so that unrelated wasmtiny processes cannot collide.

#### Scenario: Concurrent processes allocate regions
- **WHEN** two wasmtiny processes allocate shared regions concurrently
- **THEN** neither allocation SHALL fail due to a name collision with the other process

## MODIFIED Requirements

### Requirement: Explicit Detach Failure Semantics
The runtime SHALL reject accesses to unmapped shared pages via bounds checking. Access rejection SHALL be enforced by the runtime's software checks (not by relying on OS signal delivery), so trap behaviour does not depend on process-wide signal-handler state.

#### Scenario: Access after detach
- **WHEN** an instance attempts to access a shared region after its mapping has been detached via `free_region`
- **THEN** the access SHALL trap because the address range is no longer valid for that memory
