## 1. OSR Infrastructure

- [x] 1.1 Add call counter to JitCompiler for tracking function invocations
- [x] 1.2 Implement hot function detection threshold (1000 calls)
- [x] 1.3 Add OSR candidate queue for functions awaiting optimization
- [x] 1.4 Create OsrContext struct for managing OSR state

## 2. OSR Compilation

- [ ] 2.1 Add background compilation thread/async task for OSR
- [ ] 2.2 Implement optimized compilation path in JitCompiler
- [ ] 2.3 Add compilation queue for OSR candidates
- [ ] 2.4 Create OSR entry point generation

## 3. OSR State Transfer

- [ ] 3.1 Implement stack frame metadata extraction
- [ ] 3.2 Create state transfer mechanism for locals
- [ ] 3.3 Implement operand stack value transfer
- [ ] 3.4 Add control frame preservation for OSR

## 4. OSR Transition

- [ ] 4.1 Implement transition trampoline at call boundaries
- [ ] 4.2 Add code patching mechanism for OSR
- [ ] 4.3 Create jump buffer for OSR transition
- [ ] 4.4 Integrate OSR with JitRuntime execution

## 5. Integration & Testing

- [ ] 5.1 Add OSR configuration to JitRuntime
- [ ] 5.2 Integrate hot detection with interpreter execution
- [ ] 5.3 Add unit tests for OSR infrastructure
- [ ] 5.4 Add integration tests for OSR transitions