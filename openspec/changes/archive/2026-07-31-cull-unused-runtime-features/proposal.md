# cull-unused-runtime-features

## Why

wasmtiny exists solely to serve Selium (`../selium/arch3`), which consumes it with `default-features = false` (interpreter-only) and drives a narrow API: `WasmApplication::{with_store, load_module_from_memory, register_host_function, instantiate, execute_start, call_function, attach_shared_region, detach_shared_region}`, `HostFunc`/`HostCaller::memory`, the `Store` shared-region APIs, `Memory::{read,write,read_u32,write_u32}`, `RegionProt`, `FunctionType`, `WasmValue` (i32/i64 + byte codecs), and `WasmError`. A repo-wide trace confirms Selium never references anything else: no JIT, no AOT types, no suspension (`RuntimeSuspender`/`SuspendedHandle`/`HostCallOutcome`), no safepoints, no metering (`InstanceLimits`/`InstanceStats`), no snapshots, no export accessors, no direct `loader`/`interpreter`/`signal_handler` usage. Those ported features are ~14.5k lines of Rust tech debt — including 4,142 lines of *ungated* fast-JIT scaffolding referenced by nothing, a 1,763-line snapshot/restore subsystem that is both test-only and broken, and per-instruction metering that costs two lock acquisitions on the interpreter hot path while serving no consumer.

## What Changes

All changes are **BREAKING** to the public API; Selium requires no code changes (verified against `arch3`).

### Remove execution engines that will never run

- Delete `src/jit/` entirely (10,417 lines): the LLVM JIT (`llvm_backend`, `llvm_runtime`, `wasm_to_llvm`; ~108 `unsafe`) and the always-compiled fast-JIT scaffolding (`compiler`, `emitter`, `regalloc`, `runtime` — x86-64 byte emitter, never referenced, cannot execute anything).
- Delete the `llvm-jit` cargo feature, the optional `llvm-sys` dependency, `ExecutionMode`, `LlvmJit`, and all ~24 `#[cfg(feature = "llvm-jit")]` touchpoints in `application.rs`, `aot_runtime/runtime.rs`, `suspend.rs`, `lib.rs`.
- Rename the misnamed `aot_runtime` module to `engine`: it is the core runtime wrapping the interpreter (there is no AOT compilation anywhere). `AotRuntime`→`Engine`, `AotModule`→`LoadedModule` (or similar accurate names), `AotLoader`→`EngineLoader`, `AotExport`→`Export`. Remove the WAMR-ism `native_functions`/`NativeFunc`/`call_native` concept (no execution-path consumer).

### Remove dormancy subsystems Selium replaces

- **Suspension/safepoints**: delete `src/runtime/suspend.rs` (~1,001 lines), `RuntimeSuspender`, `SuspendedHandle`, `SuspensionError`, `SuspensionKind`, `is_suspension_error`, `HostCallOutcome`, `HostFunc::call_with_suspension`, and the interpreter safepoint machinery (`SafepointConfig`, suspender checks in `exec.rs`). Selium implements cooperative re-entry via `__selium_guest_poll` + mailbox writes. `HostFunc` keeps only `call` + `function_type`.
- **Metering/limits**: delete `src/runtime/metering.rs`, `InstanceMeter`, `InstanceLimits`, `InstanceStats`, `set_instance_limits`/`instance_stats`/`instance_limits`, the per-opcode `record_execution` charge points in `exec.rs` (removes two lock acquisitions per instruction), and the meter hooks in `Memory::grow`. Selium computes metering host-side (`MeteringObservation`).
- **Snapshot/restore**: delete `src/runtime/snapshot.rs` (763 lines, test-only) and the `ControlStack` serialisation support in `stack.rs` that exists only for it. Confirmed broken: module hash omits function bodies; restore panics on memory size mismatch; shared-region restore ignores recorded offset/protection; serialised frames lose `code`/`locals`.
- Delete the `sha2` dependency (only used by snapshot hashing).

### Remove dead code and dead API surface

- `src/interpreter/fast.rs` (467 lines): broken prototype (1-byte LEB assumptions, stub `Call`/`LoadGlobal`), referenced only by `#[allow(dead_code)]`.
- `src/interpreter/instructions.rs` (464 lines): `Instruction` enum never constructed; exec matches raw bytes.
- `src/loader/streaming.rs` (107 lines): O(n²) reparse-per-chunk, stringly EOF detection, zero consumers. Also drop the "streaming parse"/"incremental validation" loader requirements.
- Vestigial exception-handling/GC plumbing: tag section parsing/validation, tag imports/exports (`AotExport::Tag`, `register_tag_import`, `tag_type`), GC heap-type parsing — no EH or GC instructions exist; implies support that isn't there.
- `Module::names`/`NameSection` (never populated); drop the "Name section support" loader requirement.
- Dead methods (no caller in crate or consumer; ~30 items): `Instance::{stats, limits, set_limits, add_import, export, add_export, allocate_shared_region_standalone, attached_regions, memory_mut, table_mut, global_mut}`, `Store::{register_native_func, add_instance, get_native_func_count, instances}`, `NativeFuncRef::{call, call_with_suspension, with_name}`, `ImportType`, `ExportType::{new_table,new_global,new_tag}`, `Module::{func_at, import_count, get_func_imports, export}`, triplicated dead `matches_required`/`page_size` helpers on `MemoryType`/`TableType`/`Limits`, `ValType::{is_numeric, as_num_type, as_ref_type}`, `WasmValue::{local_func_ref, f32(), f64()}`, `SharedRegion::{ptr, is_empty}`, `ControlFrame::get_*`.
- Consumer-unused `WasmApplication` surface: `execute_main`, `register_memory_import`/`register_table_import`/`register_global_import`/`register_tag_import`, `register_function_import_binding`/`register_table_import_binding`, `imports`, `export_memory` (also a correctness hazard — deep-clones guest memory), `export_table`/`export_table_index`/`export_global`, `table`/`table_binding`/`set_table`, `func_type`/`function_binding`/`tag_type`, `instance_stats`/`instance_limits`/`set_instance_limits`.
- Replace `#[allow(dead_code)]`-kept-alive items in `suspend.rs` (going away) and elsewhere with correct `#[cfg]` gates or deletion, per AGENTS.md.

### Dependency and binary trims

- `Cargo.toml`: remove `llvm-sys` + `sha2`; make `clap`/`anyhow` optional behind a new `cli` feature with `[[bin]] required-features = ["cli"]`, so Selium's dependency tree keeps only `byteorder`, `leb128`, `libc`, `memchr`, `parking_lot`.
- Keep `src/bin/wasmtiny.rs` as a dev smoke tool (fixed to `instantiate` + `execute_start` before calling; currently it does neither).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `wasm-runtime-core`: absorbs the renamed engine (`aot_runtime` → `engine`); snapshot, suspension, and metering APIs removed; dead public API surface removed.
- `wasm-interpreter`: safepoint/suspender and metering integration removed; dead `fast`/`instructions` modules removed.
- `wasm-module-loader`: streaming/incremental-validation, name-section, and tag/typed-reference plumbing requirements removed (no EH/GC instructions exist); streaming loader deleted.

### Removed Capabilities

- `wasm-aot-runtime`: superseded — the module is renamed to `engine` under `wasm-runtime-core`; no AOT compilation exists.
- `wasm-fast-jit`: deleted — ungated dead scaffolding; never referenced.
- `llvm-jit-compiler`: deleted — unused by the sole consumer.
- `llvm-jit-runtime`: deleted — unused by the sole consumer.
- `wasm-llvm-ir-translator`: deleted — unused by the sole consumer.
- `wasm-atomics-aot`: deleted — AOT removed.
- `wasm-atomics-jit`: deleted — JIT removed.
- `runtime-metering`: deleted — Selium meters host-side.
- `instance-limits`: deleted — Selium does not configure engine limits.
- `runtime-safepoints`: deleted — suspension machinery removed.

## Impact

- **Code**: deletes ~14,500 lines of Rust across `src/jit/`, `src/runtime/{suspend,snapshot,metering}.rs`, `src/interpreter/{fast,instructions}.rs`, `src/loader/streaming.rs`, plus scattered dead methods and cfg touchpoints in `application.rs`, `aot_runtime/`, `exec.rs`, `memory.rs`, `stack.rs`, `lib.rs`.
- **Public API (BREAKING)**: removes modules `jit`, `aot_runtime` (renamed), and the suspension/metering/snapshot/safepoint types; removes `HostFunc::call_with_suspension` default method and `HostCallOutcome`; removes ~30 dead methods. Selium's used surface is untouched and requires no changes.
- **Dependencies**: `llvm-sys`, `sha2`, `clap`, `anyhow` all leave the default dependency tree (`anyhow` is used only by the CLI bin; `sha2` only by snapshot hashing). Default deps become: `byteorder`, `leb128`, `libc`, `memchr`, `parking_lot`.
- **Performance**: removes per-instruction metering locks and safepoint checks from the interpreter hot path.
- **Docs**: `README.md` feature claims updated (suspension/metering/snapshot no longer advertised); lib.rs docs lose JIT references.
- **Coordination**: apply after/alongside `remove-c-era-artifacts`; **must land before** `harden-runtime-correctness` so the hardening change only fixes code that survives.
