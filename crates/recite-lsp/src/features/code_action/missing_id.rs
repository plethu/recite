use std::collections::{BTreeMap, BTreeSet};

use lsp_types::{Range, TextEdit};
use recite_core::{SourceId, SourceIdKind, SourcePosition};

use super::{CodeActionDocument, ranges_intersect};
use crate::position::source_position_to_lsp;
use crate::summary::{FileSummary, MissingIdInsertion, MissingIdKind, MissingIdSummary};

pub(super) fn edits(
    document: &CodeActionDocument<'_>,
    documents: &[CodeActionDocument<'_>],
    range: Option<Range>,
) -> Vec<TextEdit> {
    let mut occupied = occupied_ids(documents);
    let missing_ids = document
        .summary
        .missing_ids
        .iter()
        .filter(|missing| {
            range.is_none_or(|range| {
                let marker_range = crate::position::span_to_range(document.text, &missing.span);
                ranges_intersect(range, marker_range)
            })
        })
        .collect::<Vec<_>>();
    let ordinals = missing_ordinals(document.summary);

    let mut edits = Vec::with_capacity(missing_ids.len());
    for missing in missing_ids {
        let label = readable_label(missing, &ordinals);
        let generated_anchor = unique_generated_anchor(
            &mut occupied,
            document
                .summary
                .project_relative_path()
                .unwrap_or(document.uri.as_str()),
            missing,
            &label,
        );
        let position = source_position_to_lsp(document.text, missing.insertion_position);
        edits.push(TextEdit {
            range: Range {
                start: position,
                end: position,
            },
            new_text: insertion_text(
                document.text,
                missing.insertion_position,
                missing.insertion,
                &label,
                &generated_anchor,
            ),
        });
    }

    edits.sort_by(|left, right| {
        left.range
            .start
            .line
            .cmp(&right.range.start.line)
            .then_with(|| left.range.start.character.cmp(&right.range.start.character))
    });
    edits
}

fn occupied_ids(documents: &[CodeActionDocument<'_>]) -> BTreeSet<String> {
    documents
        .iter()
        .flat_map(|document| {
            document
                .summary
                .line_ids
                .iter()
                .chain(document.summary.choice_ids.iter())
                .map(|id| id.name.clone())
        })
        .collect()
}

fn missing_ordinals(summary: &FileSummary) -> BTreeMap<(u32, MissingIdKind), u32> {
    let mut counts = BTreeMap::<(u32, MissingIdKind), u32>::new();
    let mut ordinals = BTreeMap::new();
    for missing in &summary.missing_ids {
        let block_line = enclosing_block_line(summary, missing).unwrap_or(0);
        let key = (block_line, missing.kind);
        let count = counts.entry(key).or_insert(0);
        *count = count.saturating_add(1);
        ordinals.insert((missing.span.start.line(), missing.kind), *count);
    }
    ordinals
}

fn readable_label(
    missing: &MissingIdSummary,
    ordinals: &BTreeMap<(u32, MissingIdKind), u32>,
) -> String {
    if let Some(label) = &missing.label {
        return label.clone();
    }
    let kind = match missing.kind {
        MissingIdKind::Line => "line",
        MissingIdKind::Choice => "choice",
    };
    let ordinal = ordinals
        .get(&(missing.span.start.line(), missing.kind))
        .copied()
        .unwrap_or(1);
    if ordinal <= 1 {
        kind.to_owned()
    } else {
        format!("{kind}_{ordinal}")
    }
}

fn enclosing_block<'a>(
    summary: &'a FileSummary,
    missing: &MissingIdSummary,
) -> Option<&'a crate::summary::SpannedName> {
    summary
        .blocks
        .iter()
        .take_while(|block| block.span.start.line() <= missing.span.start.line())
        .last()
}

fn enclosing_block_line(summary: &FileSummary, missing: &MissingIdSummary) -> Option<u32> {
    enclosing_block(summary, missing).map(|block| block.span.start.line())
}

fn unique_generated_anchor(
    occupied: &mut BTreeSet<String>,
    path: &str,
    missing: &MissingIdSummary,
    label: &str,
) -> String {
    for salt in 0_u32.. {
        let candidate = SourceId::generated_anchor(
            path,
            source_id_kind(missing.kind),
            missing.span.start.line(),
            missing.span.start.column(),
            label,
            salt,
        )
        .to_string();
        if occupied.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded salt search must find an unused generated ID")
}

fn source_id_kind(kind: MissingIdKind) -> SourceIdKind {
    match kind {
        MissingIdKind::Line => SourceIdKind::Line,
        MissingIdKind::Choice => SourceIdKind::Choice,
    }
}

fn insertion_text(
    text: &str,
    position: SourcePosition,
    insertion: MissingIdInsertion,
    label: &str,
    anchor: &str,
) -> String {
    match insertion {
        MissingIdInsertion::AnchorOnly => return anchor.to_owned(),
        MissingIdInsertion::AtAnchor => return format!("@{anchor}"),
        MissingIdInsertion::FullId => {}
    }
    let generated_id = format!("{label}@{anchor}");
    let Some(next) = char_at_source_position(text, position) else {
        return format!(" {generated_id}");
    };
    if next.is_whitespace() {
        format!(" {generated_id}")
    } else {
        format!(" {generated_id} ")
    }
}

fn char_at_source_position(text: &str, position: SourcePosition) -> Option<char> {
    let line = text
        .split('\n')
        .nth(usize::try_from(position.line().saturating_sub(1)).ok()?)?;
    let scalar_index = usize::try_from(position.column().saturating_sub(1)).ok()?;
    line.chars().nth(scalar_index)
}
