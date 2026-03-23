## 1. AOT Runtime Completion

- [x] 1.1 Complete AOT memory management implementation
- [x] 1.2 Complete AOT table management implementation
- [x] 1.3 Complete AOT global variable access implementation
- [x] 1.4 Implement AOT trap handling and propagation
- [x] 1.5 Add AOT function call dispatch
- [x] 1.6 Add unit tests for AOT runtime

## 2. Application Entry Point

- [x] 2.1 Create wasm-application module in src/
- [x] 2.2 Implement module loading from file
- [x] 2.3 Implement module loading from memory
- [x] 2.4 Implement instance creation with imports
- [x] 2.5 Implement function execution by name
- [x] 2.6 Implement start function execution
- [x] 2.7 Add unit tests for application layer

## 3. Native Function Handling

- [x] 3.1 Extend HostFunc trait for full implementation
- [x] 3.2 Implement native function registration
- [x] 3.3 Implement parameter passing to native functions
- [x] 3.4 Implement return value handling
- [x] 3.5 Implement native function table support
- [x] 3.6 Add unit tests for native function handling

## 4. Integration

- [x] 4.1 Integrate application layer with runtime
- [x] 4.2 Create CLI entry point (main.rs)
- [x] 4.3 Test full end-to-end WASM execution
- [x] 4.4 Verify existing tests still pass

## 5. Remove C Code

- [x] 5.1 Remove core/iwasm/aot/aot_runtime.c
- [x] 5.2 Remove core/iwasm/aot/aot_loader.c
- [x] 5.3 Remove core/iwasm/common/wasm_application.c
- [x] 5.4 Remove core/iwasm/common/wasm_native.c