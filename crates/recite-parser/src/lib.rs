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
mod diagnostic_presentation;
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

/// One key/value assignment in a statement header, preserving its source
/// boundaries for editor features that need to work inside collection values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetadataAssignment<'a> {
    pub key: &'a str,
    pub value: &'a str,
    pub value_start: usize,
    pub end: usize,
}

/// Scans statement-header assignments with the same bracket and quote rules
/// used by source lowering.
#[must_use]
pub fn metadata_assignments(line: &str) -> Vec<MetadataAssignment<'_>> {
    let leading_len = line.len() - line.trim_start().len();
    let trimmed = &line[leading_len..];
    let prefix = if trimmed.starts_with("::") {
        "::"
    } else if trimmed.starts_with('>') || trimmed.starts_with('?') {
        &trimmed[..1]
    } else {
        return Vec::new();
    };

    header::fields_after_prefix(trimmed, prefix, 0, leading_len)
        .filter_map(|field| {
            let (key, value) = field.text.split_once('=')?;
            let start = leading_len + field.offset;
            Some(MetadataAssignment {
                key,
                value,
                value_start: start + key.len() + 1,
                end: start + field.text.len(),
            })
        })
        .collect()
}

/// Finds the assignment containing a byte position in a statement header.
#[must_use]
pub fn metadata_assignment_at(line: &str, byte_index: usize) -> Option<MetadataAssignment<'_>> {
    metadata_assignments(line)
        .into_iter()
        .find(|assignment| assignment.value_start <= byte_index && byte_index <= assignment.end)
}

/// Parses one metadata value with the same scalar and array rules used by
/// source lowering.
#[must_use]
pub fn parse_metadata_value(value: &str) -> Option<recite_core::SourceMetadataValue> {
    header::parse_value(value).ok()
}
