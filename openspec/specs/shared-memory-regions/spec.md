## ADDED Requirements

### Requirement: Shared Region Allocation
The runtime SHALL allow callers to allocate shared memory regions independently of an instance's private linear memory.

#### Scenario: Create shared region
- **WHEN** a caller requests a shared region with a valid size and alignment
- **THEN** the runtime SHALL create a shared region object and return an identifier for future attachment

### Requirement: Explicit Region Attachment
The runtime SHALL require instances to attach a shared region explicitly before accessing it.

#### Scenario: Attach region to instance
- **WHEN** a caller attaches a valid shared region to an instance
- **THEN** the runtime SHALL make that region accessible to the instance according to the mapping rules for shared memory

### Requirement: Cross-Instance Visibility
The runtime SHALL make writes to an attached shared region visible to other instances attached to the same region according to the documented runtime memory model.

#### Scenario: Observe shared write
- **WHEN** one attached instance writes to a shared region
- **THEN** another attached instance SHALL be able to observe the updated bytes without requiring the runtime to perform an implicit copy through a private buffer

### Requirement: Explicit Detach Failure Semantics
The runtime SHALL reject accesses through invalid or detached shared-memory mappings.

#### Scenario: Access after detach
- **WHEN** an instance attempts to access a shared region after its mapping has been detached or invalidated
- **THEN** the runtime SHALL fail the access explicitly instead of reading or writing undefined memory
