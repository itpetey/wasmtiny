## Why

The Rust rewrite currently has a partial AOT runtime implementation. We need to complete the port to fully replace the C implementation in `core/iwasm/aot/` and integrate the application entry point (`wasm_application.c`) and native function handling (`wasm_native.c`).

## What Changes

- Complete AOT runtime port: fully replace `core/iwasm/aot/aot_runtime.c` and `aot_loader.c`
- Port `wasm_application.c`: replace C entry point with Rust equivalent
- Port `wasm_native.c`: replace host function handling with Rust native function dispatch
- Integrate all components into a unified Rust-based runtime
- Remove C implementations after successful port

## Capabilities

### New Capabilities
- `wasm-application`: WASM application loading and execution entry point
- `wasm-native`: Native (host) function registration and calling

### Modified Capabilities
- `wasm-aot-runtime`: Extend existing spec to include full runtime integration

## Impact

- Replaces `src/aot_runtime/` with complete implementation
- Replaces `src/runtime/` native function handling
- Removes: `core/iwasm/aot/aot_runtime.c`, `core/iwasm/aot/aot_loader.c`, `core/iwasm/common/wasm_application.c`, `core/iwasm/common/wasm_native.c`
- No GC: reference types handled without automatic garbage collection