## Context

wasmtiny is an interpreter-based runtime (no AOT) with a loader that
validates modules, an interpreter core, mmap-backed linear memories,
and shared-memory regions across instances. Existing tests cover
correctness (vendored spec suites, regressions, malformed-module
fixtures in `tests/malformed`). Nothing currently tests *containment*
under adversarial binaries, and the existing `wasm-test-suites`
capability requires self-contained execution: `cargo test` on a fresh
clone, no external binaries, no network. This change must honor that
constraint — the corpus toolchain has to be vendored or written in-repo.

The defining constraint (see proposal.md - Why): guest payloads are
untrusted *binaries*. Fixtures cannot be assumed to come from a
cooperative compiler, so corpus fixtures are authored as raw `wat`
(where a wat assembler is vendored) or as hand-built/patched binary
files, and every judgement is made from outside the sandbox.

## Goals / Non-Goals

**Goals:**

- Judgement of escape attempts purely by externally observed behaviour
- Corpus that builds offline, honors the self-contained test constraint
- Verdicts precise enough to bisect containment regressions
- Testing build fully isolated from shipped builds

**Non-Goals:**

- Proving absence of escapes (corpus + fuzzing is a regression net,
  not a proof; external audit and the threat-model doc carry that load)
- Sandboxing the CI host beyond resource caps — wasmtiny's process
  model already constrains the runtime; the caps are belt-and-braces
  so a runtime bug degrades to a failed job
- Long-running continuous fuzzing infrastructure (CI bursts only; a
  separate long-run farm is future work)
- Any static gate on guest code — explicitly impossible by assumption

## Decisions

### Corpus fixtures are data, not tests

Each fixture is a directory: the guest binary (or a build script that
produces it deterministically), plus a manifest naming the
threat-model technique it encodes and the expected verdict. The
harness iterates manifests; fixture authors never write harness code.
Rationale: a corpus entry must not be able to weaken its own
judgement. Alternative rejected: per-fixture `#[test]` functions —
they let fixture authors assert their own success and make the
traceability matrix manual.

### Fixture binaries built without good faith

Where source is needed, fixtures are raw `wat` assembled by a
vendored assembler, or bytes emitted by a small in-repo binary-writer
helper, or committed pinned `.wasm` files for byte-level patch cases.
No fixture depends on `rustc`/LLVM producing well-behaved code — that
is the exact assumption the corpus exists to break. The `tests/malformed`
fixtures are a seed for the loader-rejection entries; new corpus
entries extend rather than duplicate them.

### Verdicts from the outside only

The harness runs each fixture in a subprocess of the testing build,
observes: exit status, runtime trap channel, timeout, and canary
state. Canaries are checked in the harness process (parent), never
inside the runtime under test: marker directories seeded with
canary files, fd counts via `/proc/self/fd` (or `kqueue`-equivalent
fallback), baseline rusage deltas. Expected verdict is part of the
manifest; a fixture "passing" by crashing when a trap was expected is
a failure, not a pass — outcome class matters.

### Watchdog as first-class classifier

Four verdicts (clean / trapped / crashed / hung) with per-fixture
timeouts enforced by the harness, not by guest cooperation. A hung
fixture fails the run. Rationale: hangs in a backpressure or gas-limit
regime are real containment results; silently treating them as flakes
is how containment regressions ship.

### Fuzzing in-tree with deterministic bursts

`cargo-fuzz`-style targets are not assumed available (self-contained
constraint); instead thin fuzz loops over the loader and validator
take input from a corpus directory, driven by a simple in-repo
mutation loop with a fixed RNG seed plus a committed seed corpus.
If/when `cargo-fuzz` is acceptable in the CI environment it can wrap
the same entry points. CI bursts run N iterations with a fixed seed;
any crash commits the input to the seed corpus and fails the job.

### Testing build via feature flag, one way

A single `security-test` cargo feature gates all instrumentation:
canary hooks, verdict emission, per-fixture resource limits. CI asserts
the flag compiles cleanly both on and off, and that default builds
contain no instrumentation. Alternative rejected — a separate test
binary duplicating runtime wiring — would drift from the shipped
runtime and test the wrong code.

### Static gates: allowlist file, not attribute soup

`unsafe` code in wasmtiny must appear in a committed, audited
allowlist file (path + reason); CI greps actual `unsafe` occurrences
against it. cargo-deny runs against the workspace. Gates apply to
`src/` and workspace members only; `tests/corpus/` is exempt by
design — hostile code lives there on purpose, which is also why the
corpus is never compiled into anything that ships.

## Risks / Trade-offs

- [Corpus encodes only known techniques; novel escapes pass] → The
  matrix makes coverage honest; fuzzing and external audit cover the
  unknown-unknowns. The spec's coverage requirement fails CI when the
  threat model grows without tests.
- [In-repo mutation fuzzer is weaker than libFuzzer] → Coverage-guided
  fuzzing is explicitly deferred; the targets are structured so a real
  fuzzer can drive them unchanged later.
- [Canaries check after the fact; a fast escape could clean up] →
  Post-run checks are a floor, not a ceiling; the resource caps bound
  what an escape can do while the process lives. Marked as a known
  limitation in the threat-model doc.
- [Fixture binaries committed as artifacts can rot or hide malicious
  content] → Every pinned binary must have a build script or byte
  recipe committed alongside it; CI can regenerate and compare hashes.
- [Feature-gated instrumentation adds an `#[cfg]` surface that can bit
  rot] → CI builds both configurations on every run.

## Migration Plan

Additive only. New `tests/corpus/` tree, new harness, new CI jobs.
Default builds and existing test suites are untouched; rollback is
deleting the CI jobs. No shipped-API or wire-format changes.

## Open Questions

- Exact resource-limit mechanism per fixture (rlimits vs cgroups vs
  in-runtime gas accounting) — resolvable during implementation per
  platform; the spec only requires hard caps.
- Whether the vendored `wat` assembler covers all fixture needs or
  whether the in-repo binary-writer helper must grow — discovered as
  fixtures are authored.
