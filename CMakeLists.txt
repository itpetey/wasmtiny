cmake_minimum_required(VERSION 3.10)
project(wasm-runtime-rust C Rust)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Release)
endif()

find_package(Rust QUIET)

if(RUST_FOUND)
    message(STATUS "Rust compiler found: ${RUSTC_VERSION}")
else()
    message(WARNING "Rust compiler not found. Rust library will not be built.")
endif()

set(RUST_WAMR_DIR ${CMAKE_CURRENT_SOURCE_DIR})

if(RUST_FOUND)
    add_custom_target(rust_build
        COMMAND cargo build --release --manifest-path ${RUST_WAMR_DIR}/Cargo.toml
        WORKING_DIRECTORY ${RUST_WAMR_DIR}
        COMMENT "Building Rust WASM runtime"
        VERBATIM
    )

    add_custom_target(rust_test
        COMMAND cargo test --manifest-path ${RUST_WAMR_DIR}/Cargo.toml
        WORKING_DIRECTORY ${RUST_WAMR_DIR}
        COMMENT "Running Rust WASM runtime tests"
        VERBATIM
    )

    add_library(wasm_runtime_core STATIC IMPORTED)
    set_target_properties(wasm_runtime_core PROPERTIES
        IMPORTED_LOCATION ${RUST_WAMR_DIR}/target/release/libwasm_runtime_core.rlib
    )

    add_library(wasm_runtime_shared SHARED IMPORTED)
    set_target_properties(wasm_runtime_shared PROPERTIES
        IMPORTED_LOCATION ${RUST_WAMR_DIR}/target/release/libwasm_runtime_core.so
    )
endif()

add_custom_target(check_rust
    COMMAND cargo check --manifest-path ${RUST_WAMR_DIR}/Cargo.toml
    WORKING_DIRECTORY ${RUST_WAMR_DIR}
    COMMENT "Checking Rust WASM runtime"
    VERBATIM
)

add_custom_target(clippy_rust
    COMMAND cargo clippy --manifest-path ${RUST_WAMR_DIR}/Cargo.toml
    WORKING_DIRECTORY ${RUST_WAMR_DIR}
    COMMENT "Running clippy on Rust WASM runtime"
    VERBATIM
)

message(STATUS "WAMR Rust Build Configuration:")
message(STATUS "  Source directory: ${RUST_WAMR_DIR}")
message(STATUS "  Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  Rust found: ${RUST_FOUND}")
