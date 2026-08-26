## ADDED Requirements

### Requirement: Host-Facing Region Wait/Notify API
`SharedMemoryRegistry` SHALL expose a public API allowing embedders to
register a waiter on a `(region_id, offset)` pair, wait on it with a
timeout, and notify waiters on a `(region_id, offset)` pair. This API
SHALL use the same per-region waiter registry that the interpreter's
`memory.atomic.wait32`/`memory.atomic.notify` use on shared ranges, so
guest and host waits interoperate on one mechanism.

#### Scenario: Guest notify wakes host waiter
- **WHEN** a host thread holds a registered waiter on `(region, offset)`
  and a guest executes `memory.atomic.notify` on the guest address
  mapping that offset
- **THEN** the host waiter SHALL wake

#### Scenario: Host notify wakes guest waiter
- **WHEN** a guest thread is parked in `memory.atomic.wait32` on a
  shared-range address and the host calls the notify API for that
  `(region, offset)`
- **THEN** the guest wait SHALL complete as woken

#### Scenario: Notify with no waiters
- **WHEN** the notify API is called for a `(region, offset)` with no
  registered waiters
- **THEN** it SHALL report zero waiters woken and SHALL NOT error

### Requirement: Lost-Wake-Safe Waiter Registration
Waiter registration SHALL be explicit and separable from waiting, so
embedders can register, re-check the shared word, and only then wait.
A notification that arrives after registration SHALL be observed by the
subsequent wait even if it arrives before the wait begins.

#### Scenario: Notify between re-check and wait is not lost
- **WHEN** a host thread registers a waiter, re-checks the shared word
  (unchanged), and a notify arrives before it enters the wait
- **THEN** the wait SHALL return as woken rather than sleeping until
  the timeout

#### Scenario: Waiter cleanup
- **WHEN** a registered waiter handle is dropped without waiting
- **THEN** the registry SHALL NOT retain the waiter entry

### Requirement: Host Wait Capability Advertisement
The engine SHALL expose a query reporting its host-wait support level:
registry-only, or registry plus platform wake emission. The elevated
level SHALL be reported only when emission is compiled in. Enablement
SHALL be a build-time decision; there SHALL be no runtime toggle, and
the reported level SHALL be identical for every registry in the process.

#### Scenario: Embedder detects support
- **WHEN** an embedder queries the support level
- **THEN** the result SHALL reflect the engine's actual compiled
  capabilities, SHALL be the same for every registry, and SHALL NOT be
  changeable at runtime by any code in the process
