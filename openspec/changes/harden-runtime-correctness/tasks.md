# Tasks: harden-runtime-correctness

Precondition: `cull-unused-runtime-features` applied. Each group ends green: `cargo fmt --all && cargo clippy -- -D warnings && cargo test` (incl. spec suite), plus Selium `cargo test` for `selium-runtime`/`selium-kernel`.

## 1. Rebuild atomics/threads to spec

- [ ] 1.1 Add an `AtomicOp` enum with spec subopcode values (notify 0x00, wait32 0x01, wait64 0x02, fence 0x03, loads 0x10–0x16, stores 0x17–0x1D, rmw 0x1E–0x4E) plus per-op metadata (operand types, result type, access width); share it between validator, interpreter, and scanner
- [ ] 1.2 Rewrite `execute_atomic_opcode` to be fallible (`Result<()>`), trap on OOB/misalignment, zero-extend narrow loads at correct widths, implement cmpxchg at spec encodings; delete every `.unwrap()` in the atomic path
- [ ] 1.3 Rewrite `validate_atomic_instruction` from the shared metadata (correct i64 signatures, cmpxchg, wait arity); keep the shared-memory requirement; handle the fence's single reserved immediate byte
- [ ] 1.4 Add the `0xFE` case to `skip_immediates` (subopcode + per-op immediates) so block scanning cannot desync
- [ ] 1.5 Fix `memory.atomic.notify`: pop (count, address) in spec order; wake distinct waiters; return true count; prune empty waiter entries
- [ ] 1.6 Fix `memory.atomic.wait32/64`: pop (timeout, expected, address); wait32 expected is i32; nanosecond timeout with saturation (no `* 1_000_000`, no overflow); register waiter, drop locks, then park; natural-alignment trap
- [ ] 1.7 Validator: require `shared ⇒ max` for memories; enforce memarg alignment checks
- [ ] 1.8 Regression tests: guest OOB atomic traps (no host panic); misaligned atomic traps; spec-encoded rmw.add/cmpxchg execute correctly; wait/notify across two memories sharing a waiter map wakes without timeout; large timeout values do not panic; run threads-proposal `.wast` cases from the vendored spec corpus

## 2. Shared memory and mmap correctness

- [ ] 2.1 Implement top-down shared mapping placement (descending cursor from reservation top); make `ptr_at`/bounds checks accept owned range OR live shared range instead of a contiguous length; audit all `len_bytes` callers
- [ ] 2.2 Reject duplicate attach of the same region to one memory; roll back mappings on partial-attach failure (munmap + no accounting)
- [ ] 2.3 Overflow-safe arithmetic: region size align-up in `usize` with a defined maximum; `offset + len` host I/O bounds checks via `checked_add`/`saturating_sub`; include PID+entropy in shm names
- [ ] 2.4 Make `Memory` allocation fallible end-to-end (`try_new` everywhere; delete the `expect` on mmap failure); make `Memory::clone` drop shared-range state (or re-attach explicitly)
- [ ] 2.5 Delete `src/signal_handler.rs`, `with_trap_handler`, and the `setjmp` arming in `exec.rs` (fallback if rejected at review: fix TLS cleanup, `libc::sigjmp_buf`, `si_addr` validation, handler chaining, alt-stack)
- [ ] 2.6 Regression tests: attach → `memory.grow` → verify zeroed new pages + intact shared data + `memory.size` correctness; grow → detach → access old shared address traps; attach → detach → `destroy_region` succeeds; duplicate attach rejected; oversized region request errors (no panic)

## 3. Loader correctness and DoS hardening

- [ ] 3.1 Accept DataCount section (id 12) in order (after elem, before code); store the count; validate data-segment indices against it
- [ ] 3.2 Validator: reject `if` without `else` when params ≠ results; reject non-zero memory indices on bulk ops; enforce `ref.func` declared rule
- [ ] 3.3 Remove count-driven pre-allocation in parser (`Vec::with_capacity` on section counts, `vec![init; table_min]`), validator and exec (`br_table`); cap expanded locals and table minimums at defined maxima; `Table::grow` clamps deltas
- [ ] 3.4 `memory.copy`/`memory.fill`: bounds-check (u64 arithmetic) before copying; no staging buffers
- [ ] 3.5 Reject overlong/out-of-range LEB immediates in reader + exec decoders; reject invalid-UTF-8 names
- [ ] 3.6 Regression tests: DataCount-bearing binary (inline WAT with `memory.init`) loads and runs; malicious-count binaries (huge type count, locals, table min, br_table) rejected without large allocations (test under a memory-limited child process or allocation-failure injection); `if`-without-`else` case rejected; `memory.copy`/`fill` OOB traps without allocation

## 4. Engine hot path, leaks, and lock discipline

- [ ] 4.1 Cache the `Instance` per `LoadedModule` at instantiate time; `call_function` reuses it; delete the per-call `Module` deep-clone (`Arc::clone` where sharing is needed)
- [ ] 4.2 Funcref/store registration keyed by `(module_id, func_idx)`, created lazily for address-taken functions, torn down with the module; assert store size stability across N calls in a test
- [ ] 4.3 `GuestFuncRefHost`: share imported tables by `Arc`; thread the caller's `Store` into nested instantiation; add the missing result-type validation on host call returns and the cross-module `call_indirect` type check
- [ ] 4.4 Enforce global lock order (registry → memory → instance → store); never hold store/instance locks across `HostFunc::call` (clone Arc, drop lock, invoke); migrate Instance/Store/memory mutexes to `parking_lot` and delete poison-handling branches and interpreter lock `.unwrap()`s
- [ ] 4.5 Fix `WasmValue::to_bytes`/`from_bytes` tag encoding (use wasm heap-type bytes consistently or matched discriminants); add all-variant round-trip tests incl. `NullRef(ExternRef)`
- [ ] 4.6 `memory.grow` at the wasm boundary maps all failure modes to `-1`; host-side `memory_grow` keeps `Result`
- [ ] 4.7 Regression tests: N repeated calls leave store sizes unchanged; guest poll-style call pattern (mutate memory/global, call again) observes persisted state; imported-table mutation through cross-module callback is visible to the exporter; concurrent attach + instance drop completes (loom or thread stress test); lock-poison paths gone (grep)

## 5. Structured errors and idiom sweep

- [ ] 5.1 Convert `WasmError` to `thiserror` with typed fields; keep `Runtime(String)`/`Instantiate(String)` shapes; add dedicated variants for unexpected-EOF and limit violations; replace stringly EOF detection
- [ ] 5.2 Add an `opcodes` module of named constants (incl. `0xFC`/`0xFE` prefixes); sweep magic opcode bytes in exec/validator/parser/scanner/const-expr evaluators
- [ ] 5.3 Unify the three const-expr evaluators into one; unify `matches_required` onto the types and delete both private copies; cache import counts on `Module`
- [ ] 5.4 Newtype `ModuleHandle` for module indices in `WasmApplication`; change `to_bytes` to return `Vec<u8>` and ship the one-line Selium patch (or a deprecated shim)
- [ ] 5.5 Private fields with invariants (drop bypassed setters); fix doc-comment placement above `#[derive]`; remove unjustified `let _ =` suppressions per AGENTS.md
- [ ] 5.6 Verify Selium `cargo test` (runtime + kernel) with zero changes beyond the optional `to_bytes` call-site patch

## 6. Final verification

- [ ] 6.1 Grep gates: no `.unwrap()` in guest-reachable paths of `exec.rs`/`memory.rs`/`shared_memory.rs`; no `#[allow(dead_code)]` without justification; no `unsafe` in `shared_memory.rs`/`memory.rs` without a SAFETY comment
- [ ] 6.2 Run full spec suite + malformed corpus + targeted regression tests; confirm no parked/skipped placeholders
- [ ] 6.3 Run Selium full test suite against the patched path dependency
- [ ] 6.4 `openspec validate harden-runtime-correctness --strict`
