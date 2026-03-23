## ADDED Requirements

### Requirement: CLI Tool with Test Suite Selection
The test CLI SHALL support selecting which test suites to run via command-line argument.

#### Scenario: Run all test suites
- **WHEN** `wamr-integration-tests` runs without arguments
- **THEN** all test suites (spec, standalone, malformed, regression) are executed

#### Scenario: Run specific test suite
- **WHEN** `wamr-integration-tests run --suite spec` is executed
- **THEN** only the spec test suite is executed

#### Scenario: Run multiple test suites
- **WHEN** `wamr-integration-tests run --suite spec --suite standalone` is executed
- **THEN** only spec and standalone test suites are executed

### Requirement: CLI Tool with Runtime Mode Selection
The test CLI SHALL support selecting runtime execution mode via command-line argument.

#### Scenario: Specify runtime mode
- **WHEN** `wamr-integration-tests run --mode aot` is executed
- **THEN** all tests execute using AOT compilation mode

#### Scenario: Mode affects only applicable tests
- **WHEN** AOT mode is selected
- **THEN** tests that require interpreter (e.g., debug tests) are skipped gracefully

### Requirement: CLI Tool Help Output
The test CLI SHALL provide clear help output documenting available options.

#### Scenario: Display help
- **WHEN** `wamr-integration-tests --help` is executed
- **THEN** help displays available subcommands, options, and example usage

### Requirement: CLI Tool Exit Codes
The test CLI SHALL return appropriate exit codes for CI integration.

#### Scenario: All tests pass
- **WHEN** all integration tests pass
- **THEN** CLI exits with code 0

#### Scenario: Any test fails
- **WHEN** one or more integration tests fail
- **THEN** CLI exits with code 1 and outputs failure summary

#### Scenario: Setup failure (binary not found)
- **WHEN** required WAMR binaries cannot be located
- **THEN** CLI exits with code 2 and displays clear error message
