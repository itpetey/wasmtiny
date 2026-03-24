## 1. OSR Infrastructure

- [x] 1.1 Add call counter to JitCompiler for tracking function invocations
- [x] 1.2 Implement hot function detection threshold (1000 calls)
- [x] 1.3 Add OSR candidate queue for functions awaiting optimization
- [x] 1.4 Create OsrContext struct for managing OSR state

## 2. OSR Compilation

- [x] 2.1 Add background compilation thread/async task for OSR
- [x] 2.2 Implement optimized compilation path in JitCompiler
- [x] 2.3 Add compilation queue for OSR candidates
- [x] 2.4 Create OSR entry point generation

## 3. OSR State Transfer

- [x] 3.1 Implement stack frame metadata extraction
- [x] 3.2 Create state transfer mechanism for locals
- [x] 3.3 Implement operand stack value transfer
- [x] 3.4 Add control frame preservation for OSR

## 4. OSR Transition

- [x] 4.1 Implement transition trampoline at call boundaries
- [x] 4.2 Add code patching mechanism for OSR
- [x] 4.3 Create jump buffer for OSR transition
- [x] 4.4 Integrate OSR with JitRuntime execution

## 5. Integration & Testing

- [x] 5.1 Add OSR configuration to JitRuntime
- [x] 5.2 Integrate hot detection with interpreter execution
- [x] 5.3 Add unit tests for OSR infrastructure
- [x] 5.4 Add integration tests for OSR transitions