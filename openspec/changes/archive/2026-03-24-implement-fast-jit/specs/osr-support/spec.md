## ADDED Requirements

### Requirement: Tiered compilation support
The system SHALL support multiple compilation tiers, allowing functions to be recompiled at a higher tier during execution.

#### Scenario: Start with baseline tier
- **WHEN** a module is first loaded and functions are compiled
- **THEN** they are compiled to baseline tier (fast compilation, moderate performance)

#### Scenario: Upgrade to optimized tier
- **WHEN** a function's call count exceeds the OSR threshold
- **THEN** the function is recompiled to optimized tier with more aggressive optimizations

### Requirement: On-Site Replacement (OSR)
The system SHALL allow seamless transition from baseline to optimized code during execution without losing state.

#### Scenario: OSR from baseline to optimized
- **WHEN** a function in baseline tier meets the OSR criteria while executing
- **THEN** the runtime triggers recompilation and transfers execution to optimized code at the next loop/backedge

#### Scenario: OSR state transfer
- **WHEN** OSR occurs during a function's execution
- **THEN** the current stack values and local variables are transferred to the new optimized code's frame

### Requirement: OSR entry points
The system SHALL support multiple OSR entry points within a function to allow optimization at different program locations.

#### Scenario: Multiple OSR entry points
- **WHEN** a function has multiple loop entry points and back edges
- **THEN** each viable entry point has an associated OSR entry in the compiled code

#### Scenario: OSR after function prologue
- **WHEN** OSR is triggered shortly after function entry
- **THEN** the entry point handles transfer of function parameters and initial locals