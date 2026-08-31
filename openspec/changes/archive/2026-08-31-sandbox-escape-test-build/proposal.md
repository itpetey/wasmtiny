## Why

wasmtiny is rated to run malicious foreign code, but CI currently
validates correctness (spec suites, regression tests), not containment.
CI must validate the containment rating: not by proving absence of
escapes (impossible), but by proving every **known** escape technique
fails closed, fuzzing the trust boundary for unknown ones, and catching
containment regressions instantly.

The load-bearing assumption: **guest payloads are untrusted binaries,
not untrusted source**. A malicious author compiles guests however best
sabotages the runtime — hand-crafted bytecode, absent bounds checks,
binary-patched modules. No static gate can be applied to guest code;
every guest-side guarantee must be established by observing runtime
behavior from outside the sandbox. Static gates apply to wasmtiny's own
code only.

## What Changes

- **Escape corpus**: versioned adversarial guest binaries under
  `tests/corpus/`, each encoding one threat-model technique (memory
  probing at linear-memory bounds, malformed/truncated sections, table
  and global index abuse, gas/loop exhaustion, host-call abuse,
  shared-region boundary probing). Fixtures built by hostile build
  scripts (raw `wat`/hand-patched binaries — deliberately not built
  with safety instrumentation) or committed as pinned binaries.
- **Canary harness**: after each corpus run, invariants are checked
  from outside the sandbox — no fd leaks, no writes outside designated
  directories (marker-file technique), no network, no child processes,
  resource baselines restored.
- **Testing build**: a `security-test` feature/entry point exposing
  canary instrumentation and assertion hooks. Never the shipped
  runtime; zero code in default builds.
- **Boundary fuzzing**: fuzz targets for the module loader, validator,
  interpreter dispatch, and shared-region mapping, run in short
  deterministic bursts in CI with a committed seed corpus.
- **Watchdog with distinct verdicts**: clean / crashed / trapped /
  hung are reported as distinct outcomes; a guest hang is a result
  (resource-management bug), not a flake.
- **Static gates on wasmtiny only**: `forbid(unsafe_code)` enforcement
  with an audited allowlist, cargo-deny, clippy subset, plus
  deterministic pinned-toolchain builds.
- **Traceability matrix**: every entry in the written threat model maps
  to at least one corpus fixture or fuzz target; a new threat-model
  item fails CI until a test exists.

## Capabilities

### New Capabilities

- `sandbox-escape-testing`: the escape corpus, canary harness, testing
  build, boundary fuzzing, watchdog verdicts, wasmtiny-scoped static
  gates, and the traceability matrix

### Modified Capabilities

(None — this change is read-only with respect to existing runtime
behaviour; it adds a testing capability without changing any current
requirement.)

## Impact

- New test assets: `tests/corpus/` (hostile fixtures + build scripts),
  fuzz targets with seed corpora, canary harness crate or module.
- New build surface: `security-test` feature flag on wasmtiny
  (cfg-gated, absent from default builds).
- CI: new workflow jobs (corpus run, fuzz burst, static gates) with
  per-job resource caps and timeouts.
- Documentation: threat-model document becomes load-bearing (source of
  truth for the traceability matrix).
- No changes to the public runtime API or default-build behaviour.
