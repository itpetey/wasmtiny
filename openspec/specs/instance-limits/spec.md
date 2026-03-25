## ADDED Requirements

### Requirement: Configurable Instance Limits
The runtime SHALL allow callers to configure per-instance resource limits when creating or starting an instance.

#### Scenario: Apply instance budget
- **WHEN** a caller starts an instance with configured runtime limits
- **THEN** the runtime SHALL associate those limits with that instance for future enforcement

### Requirement: Memory Limit Enforcement
The runtime SHALL reject or trap memory growth that exceeds an instance's configured memory budget.

#### Scenario: Exceed memory budget
- **WHEN** an instance attempts to grow memory beyond its configured limit
- **THEN** the runtime SHALL fail the operation explicitly

### Requirement: Execution Budget Enforcement
The runtime SHALL stop execution when an instance exceeds its configured execution budget.

#### Scenario: Exceed execution budget
- **WHEN** an instance consumes more execution budget than configured
- **THEN** the runtime SHALL terminate or trap execution with an explicit budget-exceeded failure
