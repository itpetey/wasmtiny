## ADDED Requirements

### Requirement: Interpreter executes integer SIMD operations
The interpreter SHALL correctly execute all integer SIMD operations.

#### Scenario: i16x8.add_saturate_s
- **GIVEN** vectors with values [32700, 32700, 0, 0] and [100, 100, 100, 100]
- **WHEN** i16x8.add_saturate_s executes
- **THEN** result SHALL be [32767, 32767, 100, 100] (saturated)

#### Scenario: i32x4.min_u
- **GIVEN** vectors with values [5, 10, 15, 20] and [8, 3, 18, 12]
- **WHEN** i32x4.min_u executes
- **THEN** result SHALL be [5, 3, 15, 12]

### Requirement: Interpreter executes floating-point SIMD operations
The interpreter SHALL correctly execute all floating-point SIMD operations.

#### Scenario: f32x4.abs
- **GIVEN** v128 with f32 lanes [-1.0, 2.5, -0.0, inf]
- **WHEN** f32x4.abs executes
- **THEN** result SHALL be [1.0, 2.5, 0.0, inf]

#### Scenario: f32x4.neg
- **GIVEN** v128 with f32 lanes [1.0, -2.5, 0.0, -inf]
- **WHEN** f32x4.neg executes
- **THEN** result SHALL be [-1.0, 2.5, 0.0, inf]

#### Scenario: f32x4.pmin
- **GIVEN** vectors with values [1.0, 3.0, 5.0, 7.0] and [2.0, 0.0, 6.0, 8.0]
- **WHEN** f32x4.pmin executes
- **THEN** result SHALL be [1.0, 0.0, 5.0, 7.0] (lane-wise minimum, unordered)

### Requirement: Interpreter executes SIMD shifts
The interpreter SHALL correctly execute SIMD shift operations.

#### Scenario: i8x16.shl
- **GIVEN** v128 with bytes [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128]
- **WHEN** i8x16.shl with shift amount 1 executes
- **THEN** result SHALL be [2, 4, 8, 16, 32, 64, 128, 0, 2, 4, 8, 16, 32, 64, 128, 0]

### Requirement: Interpreter handles NaN correctly
The interpreter SHALL handle NaN propagation per the WebAssembly spec.

#### Scenario: f32x4.add with NaN input
- **GIVEN** vector with f32 lanes [NaN, 1.0, 2.0, 3.0]
- **WHEN** f32x4.add executes with vector [0.0, 0.0, 0.0, 0.0]
- **THEN** result lane 0 SHALL be NaN
