//! Ahead-of-Time (AOT) runtime for WebAssembly.
//!
//! This module provides the AOT runtime which manages compiled WebAssembly modules.
//! The AOT runtime handles module loading, instantiation, export resolution, and
//! function invocation.
//!
//! # Components
//!
//! - [`AotRuntime`] - Main runtime for managing AOT modules
//! - [`AotLoader`] - Loads WebAssembly modules into the AOT runtime

pub use loader::AotLoader;
pub use runtime::AotRuntime;

/// Loading support for ahead-of-time modules.
pub mod loader;
/// Runtime-related APIs.
pub mod runtime;
