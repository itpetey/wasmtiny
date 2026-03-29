## ADDED Requirements

### Requirement: AOT compiler handles SIMD operations
The AOT compiler SHALL correctly compile SIMD operations to native code.

#### Scenario: AOT i32x4.add produces correct result
- **GIVEN** WebAssembly module with i32x4.add
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** result SHALL be correct

#### Scenario: AOT f64x2.sqrt produces correct result
- **GIVEN** WebAssembly module with f64x2.sqrt
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** result SHALL be correct

#### Scenario: AOT v128.load produces correct result
- **GIVEN** WebAssembly module with v128.load
- **WHEN** compiled to AOT
- **AND** executed
- **THEN** result SHALL be correct bytes from memory

### Requirement: AOT produces optimized SIMD code
The AOT compiler SHALL produce optimized SIMD code.

#### Scenario: AOT uses native SIMD instructions
- **GIVEN** SIMD operations compiled to AOT
- **WHEN** examining compiled output
- **THEN** it SHALL use native SIMD instructions where available
