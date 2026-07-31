# harden-runtime-correctness

## Why

The C-to-Rust port was never audited for correctness or idiomatic usage, and a line-by-line audit of the surviving (post-cull) code found guest-reachable host crashes, a broken threads/atomics subsystem that Selium's planned `channel-wake-wait` work depends on, loader holes that reject real-world binaries, memory-safety-adjacent arithmetic flaws in the shared-memory feature Selium relies on most, and pervasive C-idioms. The flagship flow — load module → instantiate → poll `__selium_guest_poll` per event tick — currently deep-clones the entire module and rebuilds the instance on every call, leaks store entries per invocation, and can corrupt the address space when `memory.grow` meets an attached shared region.

## What Changes

Applies to the code surviving `cull-unused-runtime-features` (engine, interpreter, loader, mmap memory, shared-memory registry). Each fix ships with a targeted regression test.

### 1. Rebuild atomics/threads to spec (blocks Selium's `channel-wake-wait`)

- Re-map the 0xFE subopcode table to spec values (current map is non-standard: loads at 0x00–0x07 vs spec 0x10–0x16; `cmpxchg` at 0x1E/0x1F collides with spec `i32/i64.atomic.rmw.add`; `notify`/`wait`/`fence` at 0x37–0x39/0xFF vs spec 0x00–0x03).
- Make `execute_atomic_opcode` fallible end-to-end (~70 `.unwrap()`s): OOB atomic access must trap, never panic the host process.
- Align validator and interpreter signature tables (they currently disagree on ~14 ops: all i64 atomics typed as i32; cmpxchg missing from the validator; wait arity wrong).
- Add the missing `0xFE` case to `skip_immediates` (any atomic inside a block desyncs the block scanner → executes garbage from valid modules).
- Fix narrow i64 atomic loads (wrong widths, sign-extension; spec requires zero-extension).
- Fix `memory.atomic.notify` operand order (pops swapped), wake distinct waiters, return the true woken count.
- Fix `memory.atomic.wait32/64`: spec nanosecond timeouts (currently milliseconds × 1,000,000 with overflow), correct operand order/types (wait32 `expected` is i32 — currently a guest-triggerable panic), do not park while holding the memory mutex (deadlocks same-memory notify), enforce natural alignment traps, require `shared ⇒ max present` in validation.

### 2. Fix shared-memory vs growth address-space corruption (Selium-critical)

- `memory.grow` after `attach_shared_region` currently mprotects over the shared mapping (shared regions are placed at the first page above owned pages): the "new" page is unzeroed (spec violation), read-only region protection is defeated, and a later detach leaves a `PROT_NONE` hole inside `[0, len)` whose reads SIGSEGV on a path with no trap handler → host process crash. Place shared mappings so owned growth can never alias them (e.g. top-down from the end of the reserved VA range).
- Reject duplicate attach of the same region to one memory (double-attach skews `attachment_count` and permanently blocks `destroy_region`).
- Overflow-safe arithmetic: u32 align-up of host-controlled region sizes (debug panic / release wrap above ~4 GiB); unchecked `offset + len` overflow that guards `unsafe ptr::copy_nonoverlapping` in host region read/write.
- Clean up mappings on partial-attach failure; make `Memory::clone` semantics safe (currently copies shared-range metadata without the mappings → dangling bounds window); make `Memory` allocation failure an `Err`, not `expect` (OOM aborts the host); include PID/entropy in shm names (cross-process collision).

### 3. Fix loader correctness and untrusted-input DoS vectors

- Accept the DataCount section (id 12) — its rejection makes real LLVM/Binaryen binaries using `memory.init`/`data.drop` fail to load at all.
- Validator: reject `if` without `else` when params ≠ results (runtime currently fabricates result values from below the frame); reject non-zero memory indices on bulk ops; require `shared ⇒ max`; validate memarg alignment; enforce the `ref.func` declared rule.
- Stop pre-sizing allocations from untrusted counts: parser `Vec::with_capacity(section counts)`, `vec![init; table_min]` (multi-GB at parse time), validator/exec `br_table` counts, uncapped locals expansion (2³²⁻¹ locals → OOM), eager `Table::new(min)` allocation (cap table `min`, grow lazily).
- `memory.copy`/`memory.fill`: bounds-check (u64 arithmetic) before copying; never allocate a `len`-sized staging buffer (guest-controlled, up to 4 GiB).
- Decode hardening: reject overlong/out-of-range LEB immediates (silent truncation today); reject invalid-UTF8 names instead of lossy conversion.

### 4. Fix the engine hot path and leaks (Selium's poll loop)

- Cache the `Instance` per loaded module instead of rebuilding it (and deep-cloning the whole `Module`) on every `call_function` — Selium calls `__selium_guest_poll` per event-loop tick.
- Stop the per-instantiation leak of every module function into `Store::native_funcs` (unbounded growth per invocation); register funcrefs lazily/deduplicated and clean up on teardown.
- Fix cross-module `GuestFuncRefHost`: share imported tables by `Arc` (mutations currently discarded), thread the caller's `Store` through (nested instances currently get a fresh store, losing natives and the shared registry).
- Enforce one global lock order (registry → memory) between attach/detach and `Instance::drop` (ABBA deadlock today); never hold the store mutex across arbitrary `HostFunc` callbacks (non-recursive std mutex).
- Fix `WasmValue::to_bytes`/`from_bytes`: `NullRef(ExternRef)` round-trips as `FuncRef` (writes enum discriminant, reads wasm heap-type bytes) — Selium uses these codecs for entrypoint args.
- `memory.grow` failures (incl. limit failures) return `-1` per spec instead of an inconsistent hard trap; `Table::grow` clamps untrusted deltas before resize (currently ~64 GiB allocation abort).

### 5. Signal handler: remove (preferred) or fix

- Preferred: delete `src/signal_handler.rs` and the per-store `setjmp` arming in `exec.rs` — all guest-visible protections (bounds, read-only shared pages) are already enforced in software, so the process-wide SIGSEGV/SIGBUS handler is redundant, embedding-hostile (steals the host's handler without chaining), and broken (dangling freed jump buffer after the first trap; hand-sized `jmp_buf` overflows the heap on macOS-arm64/glibc-aarch64; fault address never validated, masking genuine crashes). Requirement change: detached-region access traps are guaranteed by software bounds checks, not OS signals.
- Fallback (only if removal is rejected at design review): clear TLS jump state on the trap path, use `libc::sigjmp_buf`, validate `si_addr` against armed ranges, chain the previous handler.

### 6. C-idiom and Rust-quality sweep

- `WasmError`: replace string-payload variants with `thiserror` structured variants (per AGENTS.md), keeping the `Runtime`/`Instantiate` shapes Selium matches/constructs.
- Named opcode constants module shared by loader + interpreter (replaces magic bytes in exec, const-expr evaluators, and the block scanner); unify the three const-expr evaluators and the triplicated `matches_required` helpers into one canonical implementation each.
- Concurrency hygiene: std `Mutex` → `parking_lot` uniformly (removes poison-handling divergence and interpreter `.unwrap()`-on-poison paths); cache import counts instead of O(imports) lookups per call instruction.
- API hygiene: make fields with invariants private (or drop the bypassed setter methods); newtype the `u32` module index (`ModuleHandle`); fix doc-comment placement (between `#[derive]` and item) and name-echo docs; out-parameter `to_bytes(&mut Vec<u8>)` → return `Vec<u8>` (coordinate with Selium); honour AGENTS.md (`no let _ =` suppression, `todo!()` for stubs).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `wasm-atomics`: spec-compliant subopcode encoding; validator/interpreter signature agreement; all atomic execution traps instead of panicking; correct narrow-load widths and zero-extension.
- `wasm-threads`: spec-correct wait/notify semantics (nanosecond timeouts, operand order, no lock-held parking, true woken count, alignment traps, shared⇒max validation).
- `wasm-interpreter`: untrusted-input hardening (no count-driven pre-allocation; bounds-check before copy); block scanner handles atomic immediates; `memory.grow` failure semantics per spec.
- `wasm-module-loader`: DataCount section accepted; validator correctness holes closed; allocation caps for untrusted declarations.
- `mmap-backed-memory`: growth never aliases shared mappings; fallible allocation; defined clone semantics; unix-only support explicit.
- `shared-memory-regions`: overflow-safe size arithmetic and host I/O bounds checks; cross-process-unique region names.
- `shared-region-mapping`: duplicate attach rejected; placement decoupled from linear-memory growth; partial-failure cleanup; trap guarantee via software bounds checks.
- `wasm-runtime-core`: per-call instance/module cloning eliminated; store funcref leak fixed; cross-module aliasing correctness; value codec round-trip correctness; structured errors.

## Impact

- **Code**: `src/interpreter/exec.rs` (atomics rebuild, scanner, bulk ops), `src/loader/{parser,validator,reader}.rs`, `src/memory.rs`, `src/runtime/{instance,shared_memory,values,types,error}.rs`, `src/engine/` (renamed from `aot_runtime`), `src/signal_handler.rs` (deleted), `src/lib.rs` exports.
- **Public API**: `WasmError` variants gain structured fields (Selium constructs `WasmError::Runtime(String)` and matches `Instantiate(String)` — keep those two shapes compatible); `to_bytes` signature change requires a one-line Selium update (or keep the out-param overload); the rest is behavioural-fix only.
- **Behaviour**: guest OOB atomics and malformed modules trap/error instead of panicking the host; previously-loading modules keep loading; DataCount-bearing binaries now load (new capability); shared-region guests can grow memory safely.
- **Performance**: removes per-call module deep-clone + instance rebuild (Selium tick path), per-instruction bookkeeping (with the cull), and count-driven allocations.
- **Tests**: targeted regression tests per fix (new home under `tests/`, per `remove-c-era-artifacts`); spec suite (`threads` proposal `.wast` files where applicable) plus Selium's runtime/kernel suites as integration verification.
- **Coordination**: requires `cull-unused-runtime-features` to land first (fixes apply to surviving code only); independent of `remove-c-era-artifacts` except test placement.
