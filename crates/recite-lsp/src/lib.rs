//! Language server scaffolding for Recite source files.
//!
//! This crate exposes the stdio entry point used by the `recite-lsp` binary and
//! keeps protocol handling separate from parser/compiler diagnostics. Editor
//! integrations normally launch the binary; tests and embedding hosts may call
//! [`run_stdio`] directly.
//!
//! The LSP projects syntax and schema-loading diagnostics into editor-facing
//! notifications and reuses compiler validation for schema-backed semantic
//! diagnostics. This crate projects language semantics into editor features; it
//! does not own those semantics.
//!
//! # Example
//!
//! ```no_run
//! fn main() -> Result<(), recite_lsp::ServerError> {
//!     recite_lsp::run_stdio()
//! }
//! ```

#[cfg(feature = "bench-support")]
#[doc(hidden)]
pub mod bench_support;
mod capabilities;
mod diagnostics;
mod documents;
pub(crate) mod features;
mod paths;
mod position;
mod server;
mod summary;
mod workspace;

pub use server::{ServerError, run_stdio, run_stdio_with_catalog, run_stdio_with_locale};

#[cfg(test)]
mod tests;
