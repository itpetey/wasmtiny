## 1. AOT Runtime Completion

- [ ] 1.1 Complete AOT memory management implementation
- [ ] 1.2 Complete AOT table management implementation
- [ ] 1.3 Complete AOT global variable access implementation
- [ ] 1.4 Implement AOT trap handling and propagation
- [ ] 1.5 Add AOT function call dispatch
- [ ] 1.6 Add unit tests for AOT runtime

## 2. Application Entry Point

- [ ] 2.1 Create wasm-application module in src/
- [ ] 2.2 Implement module loading from file
- [ ] 2.3 Implement module loading from memory
- [ ] 2.4 Implement instance creation with imports
- [ ] 2.5 Implement function execution by name
- [ ] 2.6 Implement start function execution
- [ ] 2.7 Add unit tests for application layer

## 3. Native Function Handling

- [ ] 3.1 Extend HostFunc trait for full implementation
- [ ] 3.2 Implement native function registration
- [ ] 3.3 Implement parameter passing to native functions
- [ ] 3.4 Implement return value handling
- [ ] 3.5 Implement native function table support
- [ ] 3.6 Add unit tests for native function handling

## 4. Integration

- [ ] 4.1 Integrate application layer with runtime
- [ ] 4.2 Create CLI entry point (main.rs)
- [ ] 4.3 Test full end-to-end WASM execution
- [ ] 4.4 Verify existing tests still pass

## 5. Remove C Code

- [ ] 5.1 Remove core/iwasm/aot/aot_runtime.c
- [ ] 5.2 Remove core/iwasm/aot/aot_loader.c
- [ ] 5.3 Remove core/iwasm/common/wasm_application.c
- [ ] 5.4 Remove core/iwasm/common/wasm_native.c