## ADDED Requirements

### Requirement: Spec test fixtures are vendored
The project SHALL include WebAssembly core spec test files as vendored fixtures rather than downloading at runtime.

#### Scenario: Fixtures exist in repository
- **GIVEN** the `tests/spec/fixtures/` directory
- **WHEN** listing files
- **THEN** it SHALL contain `core/` directory with spec test files

### Requirement: WAMR compatibility patches applied
The fixtures SHALL have WAMR compatibility patches from the legacy test suite applied.

#### Scenario: Apply thread proposal patches
- **GIVEN** `thread_proposal_ignore_cases.patch` from `tests/wamr-test-suites/spec-test-script/`
- **WHEN** building fixtures
- **THEN** it SHALL be applied to relevant test files

#### Scenario: Apply GC patches
- **GIVEN** `gc_ignore_cases.patch` and `gc_array_fill_cases.patch` from legacy suite
- **WHEN** building fixtures
- **THEN** they SHALL be applied to relevant test files

#### Scenario: Apply exception handling patches
- **GIVEN** `exception_handling.patch` from legacy suite
- **WHEN** building fixtures
- **THEN** it SHALL be applied to relevant test files

### Requirement: Fixtures are immutable
The vendored fixtures SHALL be treated as immutable - not modified during test execution.

#### Scenario: Fixtures are read-only during tests
- **GIVEN** spec test fixtures exist
- **WHEN** running tests
- **THEN** no fixture files SHALL be modified

### Requirement: Fixture version tracking
The fixtures SHALL include version information to track which upstream spec version they came from.

#### Scenario: Version manifest exists
- **GIVEN** fixtures are vendored
- **WHEN** checking the fixture directory
- **THEN** a version manifest SHALL exist with upstream commit/branch info
