//! Language server scaffolding for Recite source files.
//!
//! This crate exposes the stdio entry point used by the `recite-lsp` binary and
//! keeps protocol handling separate from parser/compiler diagnostics. Editor
//! integrations normally launch the binary; tests and embedding hosts may call
//! [`run_stdio`] directly.
//!
//! The LSP projects parser, compiler, schema, and project diagnostics into
//! editor-facing notifications. It does not own language semantics.
//!
//! # Example
//!
//! ```no_run
//! fn main() -> Result<(), recite_lsp::ServerError> {
//!     recite_lsp::run_stdio()
//! }
//! ```

mod capabilities;
mod diagnostics;
mod documents;
mod paths;
mod position;
mod server;
mod summary;
mod workspace;

pub use server::{ServerError, run_stdio};

#[cfg(test)]
mod tests;
