//! WebAssembly module loading and validation.
//!
//! This module provides utilities for parsing and validating WebAssembly binary
//! modules. It supports both streaming (incremental) and complete module parsing.
//!
//! # Components
//!
//! - [`Parser`] - High-level WebAssembly module parser
//! - [`BinaryReader`] - Low-level binary format reader
//! - [`StreamingParser`] - Streaming parser for large modules
//! - [`Validator`] - WebAssembly module validator

/// Binary WebAssembly parser support.
pub mod parser;
/// Binary reader APIs.
pub mod reader;
/// Streaming parsing APIs.
pub mod streaming;
/// Validation APIs.
pub mod validator;

pub use parser::Parser;
pub use reader::BinaryReader;
pub use streaming::{ParseState, StreamingParser};
pub use validator::Validator;
