# Tasks: Sandbox Escape Test Build

## 1. Threat model and traceability

- [x] 1.1 Write the threat-model document enumerating escape techniques (OOB memory, malformed modules, host-call abuse, resource exhaustion, shared-region probing) and verify it is committed as the source of truth for coverage
- [x] 1.2 Build the traceability-matrix generator (maps threat-model entries to corpus fixtures/fuzz targets) and verify it fails when any entry is uncovered

## 2. Testing build surface

- [x] 2.1 Add the `security-test` cargo feature gating instrumentation, verdict emission, and per-fixture resource hooks; verify `cargo build` and `cargo build --features security-test` both succeed on a clean checkout
- [x] 2.2 Verify default builds contain no instrumentation: `cargo build` then check the binary for absence of test-hook symbols (or equivalent artifact inspection)

## 3. Corpus harness

- [x] 3.1 Implement the fixture manifest format (binary source/recipe, threat-model entry, expected verdict) and the corpus iterator; verify a manifest with a missing binary fails with a clear error
- [x] 3.2 Implement subprocess execution with the four-verdict watchdog (clean/trapped/crashed/hung) and per-fixture timeouts; verify each verdict class is produced by a deliberately constructed fixture in a smoke corpus
- [x] 3.3 Implement post-run canaries: fd-leak count, marker-directory write detection, child-process detection, resource baseline (rusage) comparison; verify each canary fires on a deliberately leaking/trapping harness self-test
- [x] 3.4 Implement per-fixture hard resource caps; verify a runaway fixture (memory-grow spam) is bounded and reported as a failed job, not a hung runner

## 4. Hostile fixture toolchain and corpus

- [x] 4.1 Vendor or implement the minimal offline fixture toolchain (wat assembler or in-repo binary writer); verify fixtures build on a fresh clone with no network and no external binaries (self-contained constraint)
- [x] 4.2 Author memory-boundary fixtures (OOB loads/stores at linear-memory edges, table/global index abuse, excessive memarg alignment) with expected verdict `trapped`; verify all pass via the harness
- [x] 4.3 Author malformed-module fixtures (truncated sections, invalid LEB128, bad lengths — extend `tests/malformed`) with expected verdict `trapped` at load; verify all pass
- [x] 4.4 Author host-call abuse fixtures (adversarial argument values, type-confusion patterns) with expected verdict `trapped` or `clean-denied`; verify all pass
- [x] 4.5 Author resource-exhaustion fixtures (infinite loop, deep recursion, memory grow spam) with expected verdict `trapped`/`hung`; verify limits enforce within configured budget
- [x] 4.6 Author shared-region boundary-probing fixtures; verify cross-instance accesses outside granted regions are denied or trap
- [x] 4.7 Verify the traceability matrix passes with the full corpus (task 1.2 gate goes green)

## 5. Boundary fuzzing

- [x] 5.1 Implement fuzz entry points over loader and validator (input bytes → load outcome, no panic) driven by an in-repo deterministic mutation loop with fixed seed; verify a clean burst over a committed seed corpus exits 0
- [x] 5.2 Add interpreter-dispatch and shared-region-mapping fuzz targets; verify bursts run clean
- [x] 5.3 Verify crash discovery works end-to-end: seed a known-panic input, confirm CI fails, remove input, confirm green and corpus committed

## 6. Static gates (wasmtiny only)

- [x] 6.1 Create the audited `unsafe` allowlist (path + reason) and the CI check comparing actual occurrences against it; verify an off-list `unsafe` fails CI in a dry run
- [x] 6.2 Add cargo-deny (advisories, licenses, duplicate deps) for the workspace; verify it passes and correctly flags a test advisory
- [x] 6.3 Add clippy tier configuration; verify it runs clean on `src/` and workspace members while `tests/corpus/` is exempt

## 7. CI workflow

- [x] 7.1 Add CI jobs: corpus run, fuzz burst, static gates, dual-build check (feature on/off) with per-job resource caps and timeouts; verify a green full run on a clean checkout
- [x] 7.2 Verify failure paths: an intentionally-broken fixture, a hung fixture, and an off-list unsafe each produce distinct, attributable CI failures
