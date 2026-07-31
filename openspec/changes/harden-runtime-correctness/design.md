# Design: harden-runtime-correctness

## Context

Audited state (post-cull baseline): the interpreter, loader, mmap memory, shared-memory registry, and engine are the surviving subsystems. The audit found: (1) a threads/atomics implementation that is non-standard-encoding, internally inconsistent (validator vs exec), and guest-panicking (~70 `.unwrap()`s); (2) address-space corruption when `memory.grow` meets attached shared regions — a crash reachable by Selium's exact usage pattern; (3) loader holes (DataCount rejection blocks real bulk-memory binaries; `if`-without-`else` mis-validation) and untrusted-input allocation DoS vectors at every layer; (4) an engine hot path that deep-clones the module and rebuilds the instance per call, leaking store entries per invocation — on Selium's per-tick `__selium_guest_poll`; (5) a broken, redundant, embedding-hostile signal handler; (6) pervasive C-idioms (stringly errors, magic opcodes, triplicated logic, all-pub fields). Selium's planned `channel-wake-wait` work needs spec-correct wait/notify, so atomics must be rebuilt, not patched.

## Goals / Non-Goals

**Goals:**
- No guest-reachable host panic/abort/UB in the loader, interpreter, memory, or shared-registry paths.
- Atomics/threads conform to the WebAssembly threads proposal (encoding, signatures, semantics), verified against the spec test suite's threads cases where runnable.
- Shared-memory + growth combinations are memory-safe by construction.
- Engine per-call cost and registry sizes are bounded across invocations.
- Errors are structured (`thiserror`) with Selium-compatible shapes for `Runtime`/`Instantiate`.

**Non-Goals:**
- No new features (no SIMD, memory64, EH, GC, multi-memory).
- No performance work beyond removing the identified pathologies (per-call clone, metering locks already removed by the cull).
- No public API redesign beyond what the fixes require (the cull already pruned the surface).

## Decisions

### D1: Atomics rebuilt around a single spec-encoding source of truth
Introduce an `AtomicOp` enum (spec subopcode values as discriminants) decoded once and shared by validator, interpreter, and the block scanner's `skip_immediates`. The validator's signature table is generated from the same per-op metadata (operand types/result type) the interpreter executes, making validator/exec disagreement structurally impossible. Execution is fallible end-to-end (`Result<()>` with trap conversion); narrow loads zero-extend at the correct widths; alignment is checked (`addr % size != 0` → trap). The fence immediate is one reserved 0x00 byte per spec.
*Alternative considered:* patch the existing table — rejected: the table is wrong at every level (encoding, signatures, scanner, semantics); a metadata-driven rebuild is less code than the sum of patches and is what Selium's `channel-wake-wait` will build on.

### D2: Wait/notify park without holding memory locks
`wait32/64` register a waiter keyed by (memory identity, address), drop all locks, then park on a per-waiter condvar with a nanosecond deadline (clamp `i64` timeout: negative = forever; positive = `Duration::from_nanos` saturating). `notify` pops (count, address) in spec order, wakes up to `count` distinct waiters, returns the true count, and prunes empty waiter entries (fixes unbounded waiter-map growth). Alignment and shared⇒max rules enforced in the validator.
*Alternative considered:* keep WAMR's int64-for-both-widths signature — rejected: spec typing (i32 expected for wait32) removes the guest-triggerable `pop_i64` panic by construction.

### D3: Shared mappings placed top-down in the reserved VA range
`Memory` reserves `[0, max_pages)` VA with `PROT_NONE`. Owned pages grow upward from 0 via `mprotect`. Shared regions now map descending from the top of the reservation (bump allocator on a `next_shared_offset` cursor); `len_bytes` becomes `max(owned_end, shared_low_water)` — no wait: bounds must cover owned `[0, owned_end)` and shared ranges explicitly. Implementation: keep `owned_len` plus the `shared_ranges` list (already present); `ptr_at` accepts address if in owned range OR in a live shared range, rather than treating the shared extent as part of a contiguous length. This removes the aliasing class entirely (grow can never reach shared pages) and the detach-hole class (holes can only exist above `owned_len`, where bounds checks already reject).
*Alternative considered:* guard-band between owned and shared — rejected: any fixed band is arbitrary and still couples the two spaces; top-down + explicit range checks is mechanically simpler and uses VA, which is free on 64-bit.
*Consequence:* `len_bytes()` semantics change from "contiguous valid length" to "owned length"; `memory.size` already reports owned pages only (spec), and shared-range lookups go through the range list. Audit all `len_bytes` callers during implementation.

### D4: Duplicate attach rejected; partial attach rolls back
`map_shared_region` rejects a `region_id` already present in `shared_ranges`; on any post-`mmap` failure it `munmap`s what it mapped and records nothing. Registry keeps `attachment_count` == number of live mappings by construction.

### D5: Delete the signal handler (preferred path)
Remove `src/signal_handler.rs`, `with_trap_handler`, and the `setjmp` arming in `exec.rs`'s store path. Rationale: every guest-visible protection is enforced in software before memory access (bounds via `ptr_at`/`effective_address`; read-only shared pages via `check_writable`); with D3 there is no reachable raw fault left to translate. This also removes: process-wide handler theft, the dangling jump buffer, the mis-sized `jmp_buf`, the per-store `setjmp`+TLS cost, and async-signal-safety exposure. The `shared-region-mapping` spec is updated so detach-safety semantics rest on software checks.
*Fallback (only if review rejects removal):* fix S1–S5 (clear TLS on trap path, `libc::sigjmp_buf`, validate `si_addr` against armed ranges, chain previous handler, alt-stack). Documented here so the fallback is pre-approved design, not improvisation.

### D6: Cache the instance per loaded module; funcref registration becomes lazy and idempotent
`LoadedModule` (engine) owns one `Instance` built at instantiate time; `call_function` reuses it. Funcref/store registration keys on `(module_id, func_idx)` and is created only for functions whose address is actually taken (`ref.func`/export/element), and torn down with the module — no growth across invocations. Cross-module `GuestFuncRefHost` shares imported tables by `Arc` (like memories/globals) and reuses the caller's `Store`.
*Risk:* instance reuse changes semantics if callers relied on fresh state per call — Selium relies on *persisted* state (memory/globals across polls), so this matches the consumer; the existing persistence tests (`spine_repro`) cover it.

### D7: One global lock order: registry → memory → instance → store
Document and enforce via acquisition sites (attach/detach/drop/host-callback paths); never hold store/instance locks across `HostFunc::call` (clone the `Arc<dyn HostFunc>`, drop locks, then invoke). Migrate remaining `std::sync::Mutex` on Instance/Store/memories to `parking_lot` (no poisoning → delete the poison-mapping branches and interpreter `.unwrap()`s).

### D8: Structured errors with consumer-compatible shapes
`WasmError` becomes a `thiserror` enum; `Runtime(String)` and `Instantiate(String)` keep their single-string shape (Selium constructs/matches both). Other variants gain typed fields (e.g. `Trap { code: TrapCode, message: String }`, `Validation { message: String }`). Add a dedicated `UnexpectedEof`/limits error instead of stringly EOF/limit detection.

### D9: Canonicalise shared logic; name the opcodes
- New `opcodes` module of named constants (incl. `ATOMIC_PREFIX`, `FC_PREFIX`) used by exec, validator, parser, scanner, and const-expr evaluators.
- One `evaluate_const_expr` (engine reuses the interpreter's), one `matches_required` per entity (on the types, used by instance + engine; delete the two private copies).
- Cache the four import counts on `Module` at construction (kills O(imports) lookups per call instruction).
- `Table::grow`/`Table::new` cap sizes (`MAX_TABLE_SIZE`) and allocate lazily from untrusted minimums; `memory.grow` maps all failure modes to `-1` at the wasm boundary.

### D10: API hygiene where fixes force edits anyway
`ModuleHandle` newtype for module indices (replaces raw `u32` in `WasmApplication`); `to_bytes(&self) -> Vec<u8>` replacing the out-parameter (provide the change + a Selium patch in the same PR series — one-line change at two Selium call sites); fields with invariants made private with the bypassed setters removed; doc comments moved above `#[derive]`; `let _ =` suppressions removed or justified per AGENTS.md.

## Risks / Trade-offs

- [D3 changes `len_bytes` semantics] → All callers audited in-task; `memory.size` semantics (owned pages) preserved per spec; regression test: attach → grow → detach → grow sequence.
- [Atomics rebuild may still not be exercisable end-to-end until Selium lands `channel-wake-wait`] → Mitigation: spec-suite threads `.wast` cases + dedicated multi-threaded Rust tests exercising wait/notify across instances.
- [Instance reuse (D6) changes call semantics for hypothetical fresh-state users] → Only consumer wants persistence; covered by existing persistence tests; documented in rustdoc.
- [Deleting the signal handler removes trap translation for unforeseen raw faults] → Those faults currently indicate bugs (software checks precede access); a raw fault should crash loudly in tests rather than be masked as `MemoryOutOfBounds` — which is what the handler was doing (S3).
- [`thiserror` adoption changes `WasmError` shape] → `Runtime(String)`/`Instantiate(String)` preserved; Selium `cargo test` is the gate.
- [`to_bytes` signature change breaks Selium compile] → Coordinated one-line Selium patch (`config.rs`) shipped alongside; or keep a deprecated shim for one release — decide at implementation.

## Migration Plan

1. Land `remove-c-era-artifacts` and `cull-unused-runtime-features` first.
2. Apply this change in ordered groups (atomics → memory/shared → loader → engine → errors/idioms), each with regression tests and both repos' suites green.
3. Selium coordination: at most one accompanying patch (`to_bytes` call sites); no other changes expected.
4. Rollback: revert by group; atomics group is the only one touching encoding, so it is the rollback boundary for spec compatibility.

## Open Questions

- Top-down shared cursor starts at reservation top or leaves a guard gap? (Implementation detail; default: start at top, no gap — VA is free.)
- Should `memory.atomic.*` on non-shared memory remain a *validation* error (current, spec-correct) — yes; keep.
- Keep `load_module_from_file`'s `From<io::Error>` mapping now that errors are typed? (Map at call site instead.)
