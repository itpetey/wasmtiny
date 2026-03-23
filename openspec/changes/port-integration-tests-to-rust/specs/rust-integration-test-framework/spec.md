## ADDED Requirements

### Requirement: Binary Path Discovery
The test framework SHALL locate WAMR binaries (iwasm, wamrc) by searching in priority order:
1. `WAMR_BIN_PATH` environment variable (if set)
2. Build output directories (build/, out/)
3. System PATH

#### Scenario: Binary found via environment variable
- **WHEN** `WAMR_BIN_PATH=/custom/bin` is set and binaries exist there
- **THEN** framework uses binaries from `/custom/bin/iwasm` and `/custom/bin/wamrc`

#### Scenario: Binary found in build output
- **WHEN** `WAMR_BIN_PATH` is not set and project is built
- **THEN** framework discovers binaries in standard build output directories

#### Scenario: Binary not found
- **WHEN** no binary can be located
- **THEN** framework returns a clear error indicating which binary was missing

### Requirement: Process Execution Wrapper
The test framework SHALL provide a `WamrProcess` helper that wraps `std::process::Command` with:
- Configurable timeout (default: 30 seconds)
- Capture stdout and stderr
- Provide structured result with exit code, stdout, stderr

#### Scenario: Successful execution
- **WHEN** `WamrProcess::new("iwasm").arg("test.wasm").run()` is called on valid wasm
- **THEN** result contains exit code 0 and captured stdout

#### Scenario: Execution timeout
- **WHEN** execution exceeds timeout
- **THEN** process is killed and result contains timeout error

#### Scenario: Execution failure
- **WHEN** wasm execution fails
- **THEN** result contains non-zero exit code and stderr output

### Requirement: Temp Directory Management
The test framework SHALL provide temporary directory management for test isolation, automatically cleaned up on drop.

#### Scenario: Temp directory created and cleaned
- **WHEN** test creates a temp directory
- **THEN** directory exists during test execution and is removed after test completes

### Requirement: WASM Test Fixture Loading
The test framework SHALL provide helpers to locate WASM test fixture files from test data directories.

#### Scenario: Locate test fixture
- **WHEN** `wamr_test_fixtures()` returns fixture paths
- **THEN** paths point to valid .wasm files in test data directories
