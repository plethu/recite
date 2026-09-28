use crate::builder::Builder;
use crate::diagnostics::{INVALID, UNSUPPORTED};
use crate::{ImportError, ImportRequest, Location, SourceFamily};

mod ink;
mod twee;
mod yarn;

struct Line<'a> {
    number: usize,
    text: &'a str,
}
struct Node<'a> {
    name: &'a str,
    header: Line<'a>,
    unsupported_header: bool,
    lines: Vec<Line<'a>>,
}

pub(super) fn read(request: &ImportRequest<'_>, builder: &mut Builder) -> Result<(), ImportError> {
    if request.mapping.is_some() {
        return builder.issue(
            INVALID,
            builder.provenance(Location::Document, "mapping"),
            "mapping",
            "Field mappings apply only to JSON/CSV.",
        );
    }
    match request.family {
        SourceFamily::Twee => twee::read(request.source, builder),
        SourceFamily::Ink => ink::read(request.source, builder),
        SourceFamily::Yarn => yarn::read(request.source, builder),
        SourceFamily::Csv | SourceFamily::Json => {
            unreachable!("record families are dispatched before text readers")
        }
    }
}

fn unsupported_node(
    node: &Node<'_>,
    builder: &mut Builder,
    reason: &str,
) -> Result<(), ImportError> {
    builder.issue(
        UNSUPPORTED,
        builder.text_provenance(node.header.number, node.header.text)?,
        "block",
        reason,
    )?;
    for line in &node.lines {
        if !line.text.trim().is_empty() {
            builder.issue(UNSUPPORTED, builder.text_provenance(line.number, line.text)?, "held_back", "Block contains unsupported syntax; migrate it as a whole to preserve control flow.")?;
        }
    }
    Ok(())
}

fn plain(text: &str) -> bool {
    !text.is_empty()
        && !text.contains("''")
        && !text.contains([
            '{', '}', '[', ']', '\\', '<', '>', '#', '$', '\t', '*', '_', '~', '@', '/', '^', '%',
            '|', '&', '`',
        ])
}
