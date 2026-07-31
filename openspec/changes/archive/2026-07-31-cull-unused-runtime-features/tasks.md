# Tasks: cull-unused-runtime-features

Each group ends green: `cargo fmt --all && cargo clippy -- -D warnings && cargo test`.

## 1. Remove the JIT

- [x] 1.1 Delete `src/jit/` entirely (`mod.rs`, `compiler.rs`, `emitter.rs`, `regalloc.rs`, `runtime.rs`, `llvm_backend.rs`, `llvm_runtime.rs`, `wasm_to_llvm.rs`)
- [x] 1.2 Remove the `llvm-jit` feature and `llvm-sys` dependency from `Cargo.toml`; remove `pub mod jit` and the `LlvmJit` re-export from `lib.rs`
- [x] 1.3 Excise all `#[cfg(feature = "llvm-jit")]` code from `application.rs` (~17 sites): `ExecutionMode`, `set_execution_mode`, `llvm_jit` field, `validate_llvm_compatibility`, `compile_with_llvm*`, JIT branches in `call_function`/`execute_start`
- [x] 1.4 Excise all `#[cfg(feature = "llvm-jit")]` code from `aot_runtime/runtime.rs` (~6 sites): `jit_execution_active` field, `try_begin_jit_execution`, `ensure_jit_inactive_for_external_mutation`, the JIT `expect`/`panic!` sites (incl. `Drop` panic)
- [x] 1.5 Verify green; verify Selium `cargo check` passes

## 2. Rename `aot_runtime` → `engine`

- [x] 2.1 Rename the module directory and update `lib.rs` (`pub mod engine`), applying the name map: `AotRuntime`→`Engine`, `AotModule`→`LoadedModule`, `AotLoader`→`EngineLoader`, `AotExport`→`Export`, `create_aot_module_from_wasm`→`create_module_from_wasm`, `validate_aot_data`→`validate_wasm`
- [x] 2.2 Delete `NativeFunc`, `native_functions`, `register_native`, `call_native` and their tests
- [x] 2.3 Update all internal references (`application.rs`, tests) and rustdoc
- [x] 2.4 Verify green; verify Selium `cargo check` passes (Selium references no Aot* names)

## 3. Remove suspension and safepoints

- [x] 3.1 Delete `src/runtime/suspend.rs`; remove `RuntimeSuspender`, `SuspendedHandle`, `SuspensionError`, `SuspensionKind`, `is_suspension_error`, `HostCallOutcome` from `runtime/mod.rs` and `lib.rs` exports
- [x] 3.2 Remove `HostFunc::call_with_suspension` (trait default method) and simplify `HostFunc` to `call` + `function_type`; update `TypedHostImport` in the engine
- [x] 3.3 Remove the interpreter's suspender plumbing: `with_suspender`, `HostCallOutcome::Pending` handling in `exec.rs`, `Instance::call_with_suspension` (keep a plain `call` path)
- [x] 3.4 Remove `SafepointConfig` and safepoint checks from `exec.rs`/`interpreter/mod.rs` and the `lib.rs` re-export
- [x] 3.5 Rewrite/delete the suspension tests in `exec.rs` and `suspend.rs` that exercised the removed machinery
- [x] 3.6 Verify green; verify Selium `cargo test` passes (Selium never used these APIs)

## 4. Remove metering and instance limits

- [x] 4.1 Delete `src/runtime/metering.rs`; remove `InstanceMeter`, `InstanceLimits`, `InstanceStats` from exports
- [x] 4.2 Remove `record_execution`/`charge_execution` call sites in `exec.rs` and `instance.rs`; remove meter hooks in `memory.rs` (`Memory::grow`)
- [x] 4.3 Remove `Instance::{stats, limits, set_limits}` and `WasmApplication::{instance_stats, instance_limits, set_instance_limits}`
- [x] 4.4 Verify green; verify Selium `cargo check` passes

## 5. Remove snapshot/restore

- [x] 5.1 Delete `src/runtime/snapshot.rs` and its re-exports
- [x] 5.2 Delete `ControlStack::{to_bytes, from_bytes}` and related serialisation support in `interpreter/stack.rs`
- [x] 5.3 Remove the `sha2` dependency from `Cargo.toml`
- [x] 5.4 Verify green

## 6. Remove dead interpreter/loader modules and vestigial plumbing

- [x] 6.1 Delete `src/interpreter/fast.rs` and `src/interpreter/instructions.rs`; clean up `interpreter/mod.rs` (drop `#[allow(dead_code)]`)
- [x] 6.2 Delete `src/loader/streaming.rs` and its re-export
- [x] 6.3 Remove tag/EH plumbing: tag section parsing (`parser.rs`), tag validation (`validator.rs`), `Export::Tag`, `register_tag_import`, `tag_type`, `TagType`; loader rejects tag sections/imports/exports with explicit unsupported-feature errors
- [x] 6.4 Remove GC heap-type parsing (`parser.rs`, `validator.rs`, block-type decoding in `exec.rs`); reject with explicit unsupported-feature errors
- [x] 6.5 Delete `Module::names` and the `NameSection` struct
- [x] 6.6 Verify green; run full spec suite to confirm no in-scope binary regresses

## 7. Prune dead public API

- [x] 7.1 Delete from `WasmApplication`: `execute_main`, `register_memory_import`, `register_table_import`, `register_global_import`, `register_tag_import`, `register_function_import_binding`, `register_table_import_binding`, `imports`, `export_memory`, `export_table`, `export_table_index`, `export_global`, `table`, `table_binding`, `set_table`, `func_type`, `function_binding`, `tag_type`
- [x] 7.2 Delete from `Instance`: `add_import`, `export`, `add_export`, `allocate_shared_region_standalone`, `attached_regions`, `memory_mut`, `table_mut`, `global_mut`; rewrite tests that used them to public flows
- [x] 7.3 Delete from `Store`: `register_native`, `register_native_func`, `add_instance`, `get_native_func_count`, public `instances` field
- [x] 7.4 Delete types/methods: `NativeFuncRef`, `ImportType`, `ExportType::{new_table,new_global,new_tag}`, `Module::{func_at,import_count,get_func_imports,export}`, `MemoryType::{page_size,matches_required}`, `TableType::matches_required`, `Limits::matches_required`, `ValType::{is_numeric,as_num_type,as_ref_type}`, `WasmValue::{local_func_ref,f32,f64}`, `SharedRegion::{ptr,is_empty}`, `ControlFrame::{get_i32,get_i64,get_f32,get_f64}`
- [x] 7.5 Sweep remaining `#[allow(dead_code)]` attributes; replace legitimate feature-gated uses with `#[cfg]` and delete the rest
- [x] 7.6 Verify green; verify Selium `cargo test` passes

## 8. Dependency and binary trims

- [x] 8.1 Make `clap` and `anyhow` optional behind a new `cli` feature; add `[[bin]] required-features = ["cli"]`
- [x] 8.2 Fix `src/bin/wasmtiny.rs`: call `instantiate` and `execute_start` before `call_function`; support i64 args or document i32-only
- [x] 8.3 Verify default `cargo build` uses only `byteorder`, `leb128`, `libc`, `memchr`, `parking_lot`; verify `cargo build --features cli` builds the bin
- [x] 8.4 Verify Selium's `Cargo.lock` no longer contains `clap`/`anyhow`/`sha2` via wasmtiny

## 9. Docs and final verification

- [x] 9.1 Update `lib.rs` crate docs: remove JIT mode description; fix usage example to include `instantiate` before `call_function`
- [x] 9.2 Update `README.md`: remove suspension/metering/snapshot feature claims; describe the interpreter + shared-region model and Selium's cooperative re-entry pattern
- [x] 9.3 Run `openspec validate cull-unused-runtime-features --strict`
- [x] 9.4 Full gate: `cargo fmt --all --check`, `cargo clippy -- -D warnings`, `cargo test` (incl. spec suite), plus Selium `cargo test` for `selium-runtime` and `selium-kernel`
