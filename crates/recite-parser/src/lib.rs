//! Recite DSL parser and source mapping.
//!
//! The parser owns lossless source syntax, trivia, malformed regions, recovery,
//! and syntax diagnostics. Semantic validation stays in compiler-facing passes
//! over the lowered `recite-core` source model.
//!
//! Use this crate when tooling needs syntax-level access or when a caller wants
//! to parse source before handing the lowered model to `recite-compiler`.
//! Game-facing validation, schema checks, ID policy, and compiled output are
//! intentionally outside this crate.
//!
//! # Example
//!
//! ```
//! use recite_parser::parse;
//!
//! let source = concat!(
//!     ":: start default\n",
//!     "> intro_001\n",
//!     "  Hello.\n",
//!     "-> END\n",
//! );
//!
//! let parsed = parse("dialogue/start.recite", source);
//! assert!(parsed.diagnostics().is_empty());
//!
//! let lowered = parsed.lower_source_file();
//! assert!(lowered.diagnostics.is_empty());
//! assert_eq!(lowered.source_file.blocks[0].id.as_str(), "start");
//! ```

mod body;
mod condition;
mod diagnostics;
mod header;
mod layout;
mod lower;
mod markers;
mod parser;
mod source;
mod syntax;

pub use lower::LoweredSourceFile;
pub use parser::{Parse, parse};
pub use syntax::{ReciteLanguage, ReciteSyntaxKind, ReciteSyntaxNode};
