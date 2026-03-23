## ADDED Requirements

### Requirement: Spec Test Port
The integration test suite SHALL include `spec/` module covering WASM specification tests.

#### Scenario: Run spec tests via interpreter
- **WHEN** `cargo test --test integration spec` is executed with interpreter mode
- **THEN** all spec test cases in `tests/wamr-test-suites/spec-test-script/` are executed

#### Scenario: Run spec tests via AOT
- **WHEN** spec tests run with AOT mode enabled
- **THEN** tests compile wasm to AOT then execute via iwasm

#### Scenario: Spec test failure
- **WHEN** a spec test case fails
- **THEN** test output includes which test case failed and expected vs actual output

### Requirement: Standalone Test Port
The integration test suite SHALL include `standalone/` module covering standalone scenario tests.

#### Scenario: Run standalone test cases
- **WHEN** `cargo test --test integration standalone` is executed
- **THEN** all test cases in `tests/standalone/` are executed via iwasm

#### Scenario: Each standalone test uses isolated temp directory
- **WHEN** standalone test runs
- **THEN** test creates its own temp directory, runs the scenario, and cleans up

### Requirement: Malformed Input Test Port
The integration test suite SHALL include `malformed/` module covering invalid input handling.

#### Scenario: Malformed wasm is rejected gracefully
- **WHEN** malformed wasm files are executed via framework
- **THEN** iwasm returns non-zero exit code with appropriate error message

### Requirement: Regression Test Port
The integration test suite SHALL include `regression/` module covering known bug regression tests.

#### Scenario: Run regression tests
- **WHEN** `cargo test --test integration regression` is executed
- **THEN** all regression test cases in `tests/regression/ba-issues/` are executed

### Requirement: Feature Flag Support
The integration test suite SHALL support feature flags to enable/disable test modes (interp, fast-interp, jit, aot).

#### Scenario: Run tests in fast-interp mode
- **WHEN** tests run with `WAMR_MODE=fast-interp`
- **THEN** iwasm executes wasm using fast interpreter

#### Scenario: Run tests in AOT mode
- **WHEN** tests run with `WAMR_MODE=aot`
- **THEN** tests compile wasm first, then execute compiled output
