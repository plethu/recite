//! Recite DSL parser and source mapping.
//!
//! The parser owns lossless source syntax, trivia, malformed regions, recovery,
//! and syntax diagnostics. Semantic validation stays in compiler-facing passes
//! over the lowered `recite-core` source model.

mod diagnostics;
mod layout;
mod lower;
mod markers;
mod parser;
mod source;
mod syntax;

pub use lower::LoweredSourceFile;
pub use parser::{Parse, parse};
pub use syntax::{ReciteLanguage, ReciteSyntaxKind, ReciteSyntaxNode};
