# Design: cull-unused-runtime-features

## Context

wasmtiny serves exactly one consumer: Selium (`../selium/arch3`, crates `selium-runtime` + `selium-kernel`), which builds it with `default-features = false` and was traced (repo-wide grep + Cargo.lock inspection) to use only: `WasmApplication::{with_store, load_module_from_memory, register_host_function, instantiate, execute_start, call_function, attach_shared_region, detach_shared_region}`, `HostFunc::{call, function_type}`, `HostCaller::memory`, `SharedMemory`, `Store::{new, with_shared_registry, shared_memory_registry, allocate_shared_region, shared_region_len, destroy_shared_region, read_shared_region, write_shared_region}`, `SharedMemoryRegistry::attach_region`, `SharedRegionId`, `Memory::{read, write, read_u32, write_u32}`, `RegionProt`, `FunctionType::{new, empty}`, `ValType::Num`, `NumType::{I32, I64}`, `WasmValue::{I32, I64, from_bytes, to_bytes, i32, i64}`, `WasmError::{Runtime, Instantiate}`, `Result`. Everything else is candidate-dead. The audit also established that several subsystems are not merely unused but broken (snapshot) or harmful (per-instruction metering locks).

## Goals / Non-Goals

**Goals:**
- The crate contains only code that serves the interpreter-based embedder use case.
- Public API surface ≈ what Selium uses plus what tests legitimately exercise.
- `cargo build` default tree has 5 dependencies; no `unsafe`-heavy LLVM bindings.
- `cargo clippy -- -D warnings` and `cargo test` stay green at every step.

**Non-Goals:**
- No behavioural fixes to surviving code (that is `harden-runtime-correctness`); pure subtraction/rename here.
- No new features or capabilities.
- Keeping compile-time scaffolding "for future JIT/AOT" — if those return, they return as fresh designs, not preserved corpses.

## Decisions

### D1: Delete the whole `src/jit/` tree and the `llvm-jit` feature
Includes the ungated fast-JIT (`compiler.rs`, `emitter.rs`, `regalloc.rs`, `runtime.rs` — 4,142 lines, zero external references, `JitRuntime::execute` returns "not implemented") and the gated LLVM JIT (~108 `unsafe`, `llvm-sys` dep). Excise all `#[cfg(feature = "llvm-jit")]` touchpoints: `application.rs` (17 sites), `aot_runtime/runtime.rs` (6 sites incl. the `try_begin_jit_execution`/`ensure_jit_inactive_for_external_mutation` panics), `suspend.rs` (JIT state — file deleted anyway), `lib.rs` re-export. Delete `ExecutionMode`/`set_execution_mode`/`LlvmJit`.
*Alternative considered:* keep behind the feature flag — rejected: Selium never enables it; it doubles audit surface, CI matrix, and unsafe inventory for zero benefit.

### D2: Rename `aot_runtime` → `engine` with honest type names
The module is the core runtime (owns loader, loaded modules, invocation); "AOT" is WAMR heritage describing nothing here. Mapping:
- module `aot_runtime` → `engine`
- `AotRuntime` → `Engine`
- `AotModule` → `LoadedModule`
- `AotLoader` → `EngineLoader`
- `AotExport` → `Export`
- `create_aot_module_from_wasm` / `validate_aot_data` → `create_module_from_wasm` / `validate_wasm`
- Delete `NativeFunc`, `native_functions`, `register_native`, `call_native` (no execution-path consumer).

### D3: Remove suspension, metering, snapshot entirely rather than gate them
- **Suspension/safepoints** (`suspend.rs`, `SafepointConfig`, `RuntimeSuspender`, `SuspendedHandle`, `SuspensionError`, `SuspensionKind`, `is_suspension_error`, `HostCallOutcome`, `HostFunc::call_with_suspension`, suspender plumbing in `exec.rs`/`instance.rs`): Selium's async model is cooperative re-entry, and the interpreter's `HostCallOutcome::Pending` path exists only for this. `HostFunc` reduces to `call` + `function_type`. `HostCallOutcome::Complete(results)` call sites simplify to `Vec<WasmValue>`.
- **Metering/limits** (`metering.rs`, `InstanceMeter`, `InstanceLimits`, `InstanceStats`, `record_execution`, `charge_execution`, meter hooks in `Memory::grow`): Selium meters host-side. Removal deletes two lock acquisitions per executed instruction.
- **Snapshot** (`snapshot.rs`, `SnapshotPayload` & co., `ControlStack::{to_bytes, from_bytes}` in `stack.rs`, `sha2` dep): test-only and broken (hash omits code bodies; restore panics on size mismatch; shared-region restore loses offset/prot; frame serialisation drops `code`/`locals`).
*Alternative considered:* `#[cfg(feature)]`-gate each — rejected: features without consumers rot; git history preserves them if ever needed.

### D4: Delete dead interpreter/loader modules and vestigial plumbing
- `interpreter/fast.rs` + `interpreter/instructions.rs`: never constructed/referenced; also *incorrect* (1-byte LEB assumptions, stub semantics). `interpreter/mod.rs` loses the `#[allow(dead_code)] mod fast;`.
- `loader/streaming.rs`: no consumer, O(n²) reparse, stringly EOF detection.
- EH/GC plumbing: tag section (id 13) parsing/validation, `AotExport::Tag`/`Export::Tag`, `register_tag_import`, `tag_type`, GC heap-type parsing in `parser.rs`/`validator.rs`/`exec.rs` block-type decoding. No EH/GC instructions exist; the plumbing falsely implies support. Loader now rejects these with explicit unsupported-feature errors (spec delta in `wasm-module-loader`).
- `Module::names`/`NameSection` (never populated).

### D5: Prune the public API to consumer + test reality
Delete (callers verified absent in crate, tests, and Selium):
- `WasmApplication`: `execute_main`, `load_module_from_file`? — **kept**: used by `tests/malformed.rs` and the CLI. Deleted: `execute_main`, `register_memory_import`, `register_table_import`, `register_global_import`, `register_tag_import`, `register_function_import_binding`, `register_table_import_binding`, `imports`, `export_memory`, `export_table`, `export_table_index`, `export_global`, `table`, `table_binding`, `set_table`, `func_type`, `function_binding`, `tag_type`, `instance_stats`, `instance_limits`, `set_instance_limits`.
- `Instance`: `stats`, `limits`, `set_limits`, `add_import`, `export`, `add_export`, `allocate_shared_region_standalone`, `attached_regions`, `memory_mut`, `table_mut`, `global_mut` (tests currently using these are rewritten to use public flows or deleted with the suites).
- `Store`: `register_native`, `register_native_func`, `add_instance`, `get_native_func_count`, public `instances` field.
- `NativeFuncRef` (whole type), `ImportType`, `ExportType::{new_table, new_global, new_tag}`, `Module::{func_at, import_count, get_func_imports, export}`, `MemoryType::{page_size, matches_required}`, `TableType::matches_required`, `Limits::matches_required` (dedupe the live copies in a follow-up — here only the dead ones go), `ValType::{is_numeric, as_num_type, as_ref_type}`, `WasmValue::{local_func_ref, f32, f64}`, `SharedRegion::{ptr, is_empty}`, `ControlFrame::{get_i32, get_i64, get_f32, get_f64}`, `RuntimeSuspender::is_suspended`, `SuspendedHandle::pending_work`.
*Rule applied:* an item stays iff it has a caller in src/, in surviving tests, or in Selium. `TrapCode`, `Extern`, `Global`, `Table` etc. stay (used internally/by tests).

### D6: Dependency and binary trims
- Remove `llvm-sys` (optional) and `sha2` from `Cargo.toml`.
- New `cli` feature: `clap` + `anyhow` optional; `[[bin]] name = "wasmtiny", required-features = ["cli"]`. The bin is fixed to `instantiate` + `execute_start` before `call_function` (currently skips both).
- Default dependency set becomes: `byteorder`, `leb128`, `libc`, `memchr`, `parking_lot`.

### D7: Update docs to match reality
`lib.rs` docs (remove JIT mode + fix the usage example to include `instantiate`), `README.md` (drop suspension/metering/snapshot claims; describe interpreter + shared regions + Selium-driven model), `AGENTS.md` unaffected.

## Risks / Trade-offs

- [A future need for JIT/AOT/snapshots reappears] → Mitigation: git history + archived openspec changes (`2026-03-24-implement-llvm-jit`, `2026-03-26-add-snapshot-restore`, etc.) document prior designs; reintroduction would be a new change anyway.
- [External (non-Selium) users of the crate hit breaking removals] → Accepted: README explicitly states the project's direction is dictated by Selium; semver 0.1 → breaking changes are expected. Bump minor version and note removals in a changelog entry.
- [Tests referencing deleted APIs break] → Mitigation: task ordering deletes APIs and fixes call sites in the same commit per group; `cargo test` gate per group.
- [`HostFunc::call_with_suspension` removal breaks Selium] → Verified safe: Selium implements only `call`/`function_type`; the default method was never overridden.

## Migration Plan

1. Apply in commit-sized groups (see tasks): JIT → engine rename → suspension/safepoints → metering → snapshot → dead modules/plumbing → dead API → deps/bin → docs. Each group compiles + passes tests before the next.
2. Selium verification: `cargo check`/`cargo test` in `../selium/arch3` against the path dependency after each group (expected: no changes needed).
3. Rollback: revert by group.

## Open Questions

- Final names for `Engine`/`LoadedModule`/`EngineLoader` (alternatives: `Runtime`/`ModuleInstance`; chosen names avoid colliding with the existing `runtime` module) — confirm at implementation.
- Whether `WasmApplication` itself should eventually merge into `engine` — out of scope here; revisit after hardening.
