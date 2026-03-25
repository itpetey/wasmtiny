## 1. Metering Surface

- [x] 1.1 Define per-instance stats structures and public runtime query APIs.
- [x] 1.2 Add execution and memory-usage counter plumbing shared by interpreter and JIT paths.

## 2. Limit Enforcement

- [x] 2.1 Define per-instance limit configuration for memory and execution budgets.
- [x] 2.2 Enforce configured budgets in interpreter and JIT execution.
- [x] 2.3 Return explicit errors or traps when a budget is exceeded.

## 3. Validation

- [x] 3.1 Add tests for stats accuracy and monotonicity.
- [x] 3.2 Add tests for memory-limit and execution-budget enforcement.
