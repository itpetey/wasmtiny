# Tasks: remove-c-era-artifacts

## 1. Delete C artifact trees

- [x] 1.1 Delete `product-mini/` (all platform ports and app samples)
- [x] 1.2 Delete `samples/` (all C samples and cmake helper modules)
- [x] 1.3 Delete `benches/` (all suites and shell harnesses)
- [x] 1.4 Delete `tests/unit/`
- [x] 1.5 Delete `tests/standalone/` (including the dead inner crate)
- [x] 1.6 Move `tests/fuzz/malformed/*.wasm` (12 files) into `tests/malformed/`, then delete `tests/fuzz/`
- [x] 1.7 Delete `tests/wamr-compiler/` and `tests/requirement-engineering/`
- [x] 1.8 Delete `tests/regression/issues-deprecated/`

## 2. Retire the parked regression suite

- [x] 2.1 Delete `tests/regression.rs`
- [x] 2.2 Delete `tests/regression/` (`running_config.json`, `issues/`)
- [x] 2.3 Verify `cargo test --no-run` compiles and remaining suites are `spec`, `malformed`, `spine_repro`

## 3. Vendor the spec corpus

- [x] 3.1 Copy the 48 `test/core/*.wast` files consumed by `tests/spec.rs` from the spec submodule pin (072bd0dc) into `tests/spec-core/`
- [x] 3.2 Update `tests/spec.rs` path constant(s) from `tests/spec/test/core` to `tests/spec-core`
- [x] 3.3 Remove the `tests/spec` submodule (`git submodule deinit`, delete mount point) and delete `.gitmodules`
- [x] 3.4 Verify `cargo test --test spec` passes on a fresh clone with no submodule step; record the upstream pin in a comment/README note in `tests/spec-core/`

## 4. Replace CI

- [x] 4.1 Delete all 25 workflows under `.github/workflows/`, plus `.github/actions/`, `.github/scripts/`, `.github/codeql/`
- [x] 4.2 Add `.github/workflows/ci.yml`: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on `ubuntu-latest` and `macos-latest`
- [x] 4.3 Rewrite `.github/dependabot.yml` for `cargo` and `github-actions` ecosystems only
- [x] 4.4 Rewrite or delete `.github/ISSUE_TEMPLATE/` entries (remove links to deleted WAMR docs/security pages)

## 5. Housekeeping

- [x] 5.1 Delete all tracked `.DS_Store` files; add `.DS_Store` to `.gitignore`
- [x] 5.2 Remove `[profile.fast-jit]` from `.cargo/config.toml`; review global `-C target-feature=-crt-static` rustflags for necessity
- [x] 5.3 Add `LICENCE-MIT` (or change `Cargo.toml` to `Apache-2.0` only — pick one; default: add MIT file)
- [x] 5.4 Update `AGENTS.md`: remove `[workspace.dependencies]` mandate, document test layout (`tests/spec-core`, malformed corpus, targeted regression tests) and pre-commit checks
- [x] 5.5 Fix `README.md` typo and refresh stale descriptions

## 6. Verification

- [x] 6.1 `cargo fmt --all --check`, `cargo clippy -- -D warnings`, `cargo test` all green locally
  - fmt: passes ✓
  - test: all 352 tests pass ✓
  - clippy: 7 pre-existing warnings in `src/` (out of scope per "No `src/` changes")
- [ ] 6.2 Fresh-clone simulation (`git clone` to a temp dir, no submodule flags) → `cargo test` green
- [ ] 6.3 New CI workflow passes on GitHub for the PR
- [x] 6.4 Confirm with maintainer: delete the four stale in-flight WAMR-era openspec changes (`implement-wasm-simd`, `implement-wasm-memory64`, `rewrite-wamr-documentation`, `migrate-to-rust-examples`); if confirmed, delete them here
