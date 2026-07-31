# remove-c-era-artifacts

## Why

wasmtiny's `src/` is 100% Rust, but the repo still carries ~11 MB of WAMR C-era debris: platform ports and samples for a C runtime that no longer exists, benchmark scripts hardcoding deleted `iwasm`/`wamrc` binaries, 25 CI workflows that build C artifacts (NuttX/SGX/Zephyr/SDK/VSCode extension), and C/C++ test suites (googletest, libFuzzer) targeting the deleted runtime. Meanwhile the Rust test suites that matter are silently broken: the `tests/spec` submodule is not checked out so all 48 spec tests fail, and `tests/regression.rs` "passes" while running 0 of its 103 parked WAMR-issue cases — false confidence instead of signal.

## What Changes

### Deletions (no Rust replacement)

- `product-mini/` — 17 OS platform ports (alios-things … zephyr) of the deleted C `iwasm` binary + C app samples.
- `samples/` — 25 C samples demonstrating the C embed API, WASI, socket-api, SGX — all out of scope (interpreter-only, no WASI).
- `benches/` — coremark/dhrystone/jetstream/libsodium/polybench/sightglass shell harnesses; every script invokes `product-mini/platforms/*/build/iwasm` or `wamr-compiler/build/wamrc` (both deleted) and requires wasi-sdk. Unrecoverable; a future Rust bench harness can re-add workloads if wanted.
- `tests/unit/` — WAMR googletest C++ suites (aot, gc, wasm-c-api, …).
- `tests/standalone/` — C wasm-apps + `run.sh` scripts + a dead Rust crate whose manifest does not parse (`version.workspace = true` with no workspace) and which requires an `iwasm` binary.
- `tests/fuzz/` — C++/CMake libFuzzer project for the C runtime. Salvage: move `tests/fuzz/malformed/*.wasm` (12 crashers) into `tests/malformed/`.
- `tests/wamr-compiler/`, `tests/requirement-engineering/` (GC+AOT python runners), `tests/regression/issues-deprecated/` (never read by the harness).
- `.github/` in full — 25 workflows, 2 composite actions, 7 scripts, C CodeQL config; dependabot config tracks deleted directories (`.devcontainer`, `build-scripts`, `language-bindings`); ISSUE_TEMPLATE links deleted docs.

### Test-suite repairs

- **Delete the parked regression charade**: `tests/regression.rs` + `tests/regression/` (running_config.json, `issues/`). All 103 runnable-eligible cases are parked (`--heap-size` unimplemented, AOT/JIT/GC modes N/A); the suite executes zero tests. Fixed-bug coverage moves to targeted Rust regression tests (see the companion changes `cull-unused-runtime-features` / `harden-runtime-correctness`, which add tests for each bug they fix).
- **Make the spec suite self-contained**: vendor the 48 required `test/core/*.wast` files from the WebAssembly spec repo into `tests/spec-core/`, point `tests/spec.rs` at them, delete the `tests/spec` submodule and `.gitmodules`. `cargo test` then passes on a fresh clone with no network/submodule step.
- **Keep as-is**: `tests/spine_repro.rs` (4/4 passing, current), `tests/malformed.rs` (passing; gains the 12 salvaged fuzz crashers).

### New minimal Rust CI

- One workflow: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on linux + macos. No LLVM, no C toolchain, no submodule step.
- Rewrite `dependabot.yml` for cargo + github-actions ecosystems only.

### Housekeeping

- Delete 10 tracked `.DS_Store` files; add `.DS_Store` to `.gitignore`.
- Remove `[profile.fast-jit]` from `.cargo/config.toml` (WAMR leftover; JIT is removed by the companion change); review the global `-C target-feature=-crt-static` rustflags.
- Reconcile licensing: `Cargo.toml` declares `Apache-2.0 OR MIT` but only the Apache-2.0 `LICENCE` file exists — add `LICENCE-MIT` (or correct the declaration).
- Fix `AGENTS.md`: drop the `[workspace.dependencies]` mandate (this is a single crate, not a workspace); document the test layout.
- Fix `README.md` typo ("A small WebAssembly" → "A small WebAssembly runtime") and refresh feature claims (suspension/metering/snapshot claims are removed by the companion change — coordinate wording at apply time).

## Capabilities

### New Capabilities

- `wasm-test-suites`: Self-contained Rust test infrastructure — vendored core spec suite, malformed-module corpus, and targeted regression tests that run via `cargo test` on a fresh clone with no C toolchain, no external binaries, and no git submodules.

### Modified Capabilities

(none — no runtime requirement changes)

## Impact

- **Repo size**: removes ~11 MB (`product-mini` 1.1 MB, `samples` 1.4 MB, `tests/unit` 1.2 MB, `tests/standalone` 4.2 MB, `tests/fuzz` 248 KB, `tests/regression` 1.8 MB, `tests/requirement-engineering` 396 KB, `.github` 304 KB, misc) across 166 C/C++ sources, 109 CMake files, 107 shell scripts, 25 workflows.
- **Build/test**: `cargo test` becomes fully green on a fresh clone (48 spec tests currently fail on the missing submodule). Test targets shrink to `spec`, `malformed`, `spine_repro` + new targeted tests.
- **No `src/` changes**, no public API changes, no dependency changes. Selium is unaffected.
- **Git**: deletes the `tests/spec` submodule (`.gitmodules` removed); adds vendored `.wast` files (~spec corpus).
- **Coordination**: independent of `cull-unused-runtime-features`; must land before or alongside it. `harden-runtime-correctness` assumes this layout (targeted tests live alongside `tests/spec.rs`).
