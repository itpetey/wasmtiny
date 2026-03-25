## 1. Runtime State Model

- [x] 1.1 Define canonical suspended-instance state structures for interpreter and JIT execution.
- [x] 1.2 Add public suspend/resume lifecycle APIs with opaque suspended-instance handles.

## 2. Safepoint Execution

- [x] 2.1 Add cooperative safepoint checks to the interpreter execution loop.
- [x] 2.2 Add safepoint-aware entry wrappers, resume plumbing, and execution-context validation for the JIT path.
- [x] 2.3 Add explicit hostcall suspension plumbing and pending-work handles for interpreter execution, and reject unsupported JIT pending-hostcall suspension explicitly.

## 3. Validation

- [x] 3.1 Add tests for suspend/resume correctness across locals, stack state, and memory visibility.
- [x] 3.2 Add failure tests for unsupported suspension states and invalid resume attempts.
