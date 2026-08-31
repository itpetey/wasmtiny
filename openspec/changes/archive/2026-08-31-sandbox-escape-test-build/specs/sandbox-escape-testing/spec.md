## Purpose

Validates that wasmtiny contains malicious guest programs: a corpus of
adversarially-compiled guest binaries, a post-run canary harness, trust
boundary fuzzing, wasmtiny-scoped static gates, and a traceability
matrix tying every threat-model entry to a test.

## ADDED Requirements

### Requirement: Adversarial escape corpus
The repository SHALL maintain a versioned corpus of adversarial guest
binaries under `tests/corpus/`, each encoding one threat-model escape
technique. Corpus fixtures SHALL be built by hostile build scripts or
committed as pinned binaries; they SHALL NOT be assumed to contain any
safety instrumentation, bounds checks, or good faith in the compiler
toolchain that produced them.

#### Scenario: Corpus is self-contained
- **WHEN** a fresh clone is inspected and the corpus is built
- **THEN** every fixture SHALL be reproducible from committed sources
  or present as a committed binary, with no network access required

#### Scenario: Corpus covers the threat model
- **WHEN** the traceability matrix is generated
- **THEN** every threat-model entry SHALL map to at least one corpus
  fixture or fuzz target

#### Scenario: New threat-model item without a test
- **WHEN** a threat-model entry has no corresponding corpus fixture or
  fuzz target
- **THEN** CI SHALL fail with a report of the uncovered entry

### Requirement: Escape attempts fail closed
Every corpus fixture SHALL be executed and judged solely by observed
runtime behaviour: the escape attempt SHALL fail, and it SHALL fail
safely — the guest traps or is terminated, the runtime process
survives, and no capability or resource is acquired by the guest.

#### Scenario: Out-of-bounds linear memory access
- **WHEN** a guest executes a load or store whose effective address
  falls outside its linear memory
- **THEN** the runtime SHALL trap and SHALL NOT read or write host
  memory outside the guest's linear memory allocation

#### Scenario: Malformed module rejected
- **WHEN** a corpus fixture contains a malformed, truncated, or
  binary-patched module (bad section lengths, invalid LEB128, table
  and global index abuse, excessive memarg alignments)
- **THEN** the loader SHALL reject the module before instantiation
  without panicking or reading beyond the input buffer

#### Scenario: Host-call abuse
- **WHEN** a guest invokes a host import with argument values designed
  to induce out-of-bounds access or type confusion in the host
- **THEN** the host function SHALL reject the call safely and the
  runtime SHALL remain intact

#### Scenario: Resource exhaustion
- **WHEN** a guest attempts unbounded growth (infinite loops, memory
  grow spam, deep recursion, stack exhaustion)
- **THEN** the runtime SHALL enforce configured resource limits and
  terminate or trap the guest within the configured budget

### Requirement: Post-run canary checks
After each corpus execution, the harness SHALL verify host invariants
from outside the sandbox: no file descriptors leaked, no writes
outside designated sandbox directories (marker-file technique), no
outbound network connections, no child processes, and resource usage
returned to baseline.

#### Scenario: Clean corpus run
- **WHEN** the full corpus has run
- **THEN** all canary checks SHALL pass and the harness SHALL report a
  clean verdict

#### Scenario: Detected escape attempt
- **WHEN** any canary check fails (unexpected file write, fd leak,
  resource baseline not restored)
- **THEN** the harness SHALL fail CI with a report identifying the
  fixture and the violated invariant

### Requirement: Trust boundary fuzzing
Fuzz targets SHALL cover every component that parses or acts on
untrusted input: the module loader, the validator, interpreter
instruction dispatch, and shared-region mapping. CI SHALL run short
fuzz bursts with a committed seed corpus and deterministic
reproduction.

#### Scenario: Fuzzer finds no crash in CI burst
- **WHEN** the CI fuzz burst completes its configured iteration budget
- **THEN** the run SHALL exit cleanly with a reproducible seed corpus
  committed

#### Scenario: Fuzzer finds a crash
- **WHEN** a fuzz burst produces a crash, hang, or sanitizer abort
- **THEN** CI SHALL fail and SHALL emit the failing input and
  reproduction instructions

### Requirement: Distinct watchdog verdicts
The corpus runner SHALL classify each fixture outcome as exactly one
of: clean exit, trapped, crashed, or hung. A hang SHALL be treated as
a test result indicating a resource-management defect, not as flake
or infrastructure failure.

#### Scenario: Hung guest
- **WHEN** a fixture exceeds its execution timeout
- **THEN** the runner SHALL record a `hung` verdict, fail the run, and
  preserve diagnostics for the fixture

#### Scenario: Trapped guest
- **WHEN** a fixture terminates via runtime trap
- **THEN** the runner SHALL record a `trapped` verdict and continue
  with the remaining fixtures

### Requirement: Static gates scoped to wasmtiny
Static analysis gates SHALL apply to wasmtiny's own code only: unsafe
code forbidden by default with an audited allowlist, dependency and
advisory checking, and lint tiers. No static gate SHALL be applied to
guest payloads, and no security claim SHALL depend on guest code being
compiled with any particular toolchain, flags, or instrumentation.

#### Scenario: Unaudited unsafe code
- **WHEN** unsafe code appears outside the audited allowlist in
  wasmtiny sources
- **THEN** CI SHALL fail

#### Scenario: Advisory in dependency tree
- **WHEN** cargo-deny detects a disallowed dependency or known
  advisory in wasmtiny's dependency tree
- **THEN** CI SHALL fail

### Requirement: Testing build isolation
Canary instrumentation, assertion hooks, and other test-only surfaces
SHALL be compiled only into the testing build and SHALL be absent
from default and release builds of the runtime.

#### Scenario: Default build excludes test hooks
- **WHEN** the runtime is built with default features
- **THEN** no security-test instrumentation code SHALL be present in
  the binary

### Requirement: Sandboxed CI execution
Corpus and fuzz jobs SHALL execute with hard resource caps (rlimits or
equivalent) and per-job timeouts, such that a malicious guest can at
most fail the job, not the machine or other jobs.

#### Scenario: Runaway fixture
- **WHEN** a fixture attempts to exhaust runner resources
- **THEN** the job's resource caps SHALL bound the damage and the job
  SHALL fail with a diagnostic rather than hanging indefinitely
