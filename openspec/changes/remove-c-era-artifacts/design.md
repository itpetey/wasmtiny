# Design: remove-c-era-artifacts

## Context

The WAMR C core was already deleted (`core/`, `build-scripts/`, `language-bindings/`, etc. are gone), but its debris remains: 374 files under `product-mini/`/`samples/`/`benches/`, C/C++ test trees under `tests/`, and 25 CI workflows targeting C artifacts. Audit confirmed (repo-wide grep + `cargo metadata`) that nothing in the Rust build, the surviving Rust test harnesses, or the consumer (Selium) references any of it. Two Rust harnesses are currently broken in opposite ways: `tests/spec.rs` fails all 48 tests because the `tests/spec` submodule is not checked out; `tests/regression.rs` passes while executing zero tests (all 103 cases parked on WAMR-only features).

## Goals / Non-Goals

**Goals:**
- Every remaining file in the repo serves the Rust runtime or its consumer.
- `cargo test` is green on a fresh clone with no submodule/network/C-toolchain step.
- CI reflects reality: fmt + clippy + test on the platforms developers use.
- No parked/placeholder tests.

**Non-Goals:**
- No `src/` changes (covered by the companion changes).
- No new benchmark harness (a Rust bench harness may be proposed separately if Selium wants interpreter perf tracking).
- No preservation of C wasm-app corpora beyond the 12 fuzz crashers (the vendored spec suite + malformed corpus + new targeted tests provide better coverage; WAMR-era `.wasm` apps largely exercise WASI/AOT/JIT paths that no longer exist).

## Decisions

### D1: Delete, don't archive, the C trees
`product-mini/`, `samples/`, `benches/`, `tests/unit/`, `tests/standalone/`, `tests/fuzz/`, `tests/wamr-compiler/`, `tests/requirement-engineering/`, `tests/regression/issues-deprecated/`, and all of `.github/` (minus replacements) are deleted outright.
*Alternatives considered:* moving to an `attic/` directory or a separate repo — rejected: git history already preserves everything; an attic invites bit-rot and confused contributors.

### D2: Vendor the spec corpus instead of keeping the submodule
Commit the `test/core/*.wast` files that `tests/spec.rs` consumes (48 files, from WebAssembly/spec@072bd0dc) into `tests/spec-core/`, update the harness path constant, delete `.gitmodules` and the submodule mount.
*Alternatives considered:* (a) keep submodule + document/CI `git submodule update --init` — rejected: fresh clones and crates.io packaging break the suite; CI must remember the step forever; (b) fetch at build time — rejected: network dependency in tests.
*Trade-off:* vendored corpus no longer tracks upstream automatically; bumping it is a deliberate copy + diff review, which is acceptable for a runtime targeting a stable spec profile (MVP + bulk-memory + sign-extension + threads).

### D3: Delete the parked regression suite rather than un-park it
`tests/regression.rs` re-implements iwasm CLI semantics (exit codes, stdout matching, `-f` flags). 0/103 cases run; the biggest un-park lever (`--heap-size`, 34 cases) has no meaning for the mmap memory model; AOT/JIT/GC parks (53 cases) are permanent post-cull. Fixed-bug coverage moves to targeted Rust tests written alongside each fix (see `harden-runtime-correctness`).
*Alternatives considered:* implement `--heap-size` to salvage 34 cases — rejected: it would test a WAMR CLI flag, not wasmtiny behaviour; several of those cases assert WAMR-specific error strings by design.

### D4: Salvage the fuzz crash corpus
Move `tests/fuzz/malformed/*.wasm` (12 files) into `tests/malformed/`; the existing WalkDir-based harness picks them up automatically. Delete the C++ libFuzzer target — a future Rust fuzzing setup (e.g. `cargo-fuzz` against the loader) can be proposed separately.

### D5: One minimal CI workflow, platform matrix of two
Single `ci.yml`: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on `ubuntu-latest` and `macos-latest` (the dev/CI fleet; the mmap memory code is unix-only). `dependabot.yml` tracks `cargo` and `github-actions` only.
*Alternatives considered:* keeping a reduced set of WAMR workflows — rejected: every one of the 25 builds artifacts that no longer exist (verified: several reference deleted `build-scripts/`, `version.h`, `doc/` paths).

### D6: Licensing reconciliation
Add `LICENCE-MIT` alongside the existing Apache-2.0 `LICENCE`, matching `Cargo.toml`'s `Apache-2.0 OR MIT`. WAMR-derived code retains its Apache-2.0 attribution; the MIT option covers new Rust code per the declaration.
*Open question folded here:* if maintainers prefer Apache-2.0-only, change `Cargo.toml` instead — one of the two must happen.

## Risks / Trade-offs

- [Deleting benchmark suites loses perf regression signal] → The suites were unrunnable (hardcode deleted binaries); mitigated by an optional future criterion-based harness against `WasmApplication`.
- [Vendored spec corpus drifts from upstream] → Acceptable: spec profile is stable; bump procedure documented in AGENTS.md.
- [Deleting `tests/unit` `.wasm`/`.wat` apps loses potential fixtures] → They test C-runtime behaviours (shared-heap, AOT stack frames, wasm-c-api); the spec suite covers wasm semantics better. Git history preserves them.
- [Someone relied on WAMR CI for release automation] → wasmtiny has no release process yet; the minimal CI is the honest baseline to grow from.

## Migration Plan

1. Land deletions + salvages + vendored corpus + new CI in one PR (no `src/` changes, so no consumer impact).
2. Verify: fresh clone → `cargo test` green; GitHub Actions green on the new workflow.
3. Rollback: revert the PR; nothing else depends on these paths.

## Open Questions

- Keep `tests/regression/running_config.json` issue IDs as comments/names for the new targeted tests where a WAMR issue maps to a bug fixed in `harden-runtime-correctness`? (Nice-to-have traceability; default: reference issue numbers in test names where applicable.)
- Should the four stale in-flight WAMR-era openspec changes (`implement-wasm-simd`, `implement-wasm-memory64`, `rewrite-wamr-documentation`, `migrate-to-rust-examples`) be deleted in this same housekeeping pass? (Recommendation: yes — none align with Selium's requirements; confirm with maintainer.)
