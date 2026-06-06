use std::collections::{BTreeMap, BTreeSet};

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Range, TextDocumentEdit,
    TextEdit, Uri, WorkspaceEdit,
};
use recite_core::SourcePosition;

use crate::position::source_position_to_lsp;
use crate::summary::{FileSummary, MissingIdKind, MissingIdSummary};

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub(crate) struct CodeActionDocument<'a> {
    pub(crate) uri: &'a Uri,
    pub(crate) text: &'a str,
    pub(crate) summary: &'a FileSummary,
}

pub(crate) fn code_action(
    params: &CodeActionParams,
    documents: &[CodeActionDocument<'_>],
) -> Option<CodeActionResponse> {
    let document = documents
        .iter()
        .find(|document| *document.uri == params.text_document.uri)?;

    let mut actions = Vec::new();
    let include_quick_fix = includes_kind(params, &CodeActionKind::QUICKFIX);
    let include_fix_all = includes_kind(params, &CodeActionKind::SOURCE_FIX_ALL);

    let quick_fix_edits = if include_quick_fix {
        missing_id_edits(document, documents, Some(params.range))
    } else {
        Vec::new()
    };
    if quick_fix_edits.len() == 1 {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Insert missing stable ID".to_owned(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(params.context.diagnostics.clone()),
            edit: Some(workspace_edit(
                document.uri.clone(),
                document.summary.version,
                quick_fix_edits,
            )),
            ..CodeAction::default()
        }));
    }

    let fix_all_edits = if include_fix_all {
        missing_id_edits(document, documents, None)
    } else {
        Vec::new()
    };
    if !fix_all_edits.is_empty() {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Insert all missing stable IDs in file".to_owned(),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            edit: Some(workspace_edit(
                document.uri.clone(),
                document.summary.version,
                fix_all_edits,
            )),
            ..CodeAction::default()
        }));
    }

    Some(actions)
}

fn includes_kind(params: &CodeActionParams, kind: &CodeActionKind) -> bool {
    params
        .context
        .only
        .as_ref()
        .is_none_or(|kinds| kinds.iter().any(|candidate| candidate == kind))
}

fn missing_id_edits(
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
        let stem = readable_stem(document.summary, missing, &ordinals);
        let generated_id = unique_generated_id(
            &mut occupied,
            document
                .summary
                .project_relative_path()
                .unwrap_or(document.uri.as_str()),
            missing,
            &stem,
        );
        let position = source_position_to_lsp(document.text, missing.insertion_position);
        edits.push(TextEdit {
            range: Range {
                start: position,
                end: position,
            },
            new_text: insertion_text(document.text, missing.insertion_position, &generated_id),
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

fn readable_stem(
    summary: &FileSummary,
    missing: &MissingIdSummary,
    ordinals: &BTreeMap<(u32, MissingIdKind), u32>,
) -> String {
    let block = enclosing_block(summary, missing)
        .map(|block| block.name.as_str())
        .unwrap_or("dialogue");
    let kind = match missing.kind {
        MissingIdKind::Line => "line",
        MissingIdKind::Choice => "choice",
    };
    let ordinal = ordinals
        .get(&(missing.span.start.line(), missing.kind))
        .copied()
        .unwrap_or(1);
    if ordinal <= 1 {
        format!("{block}_{kind}")
    } else {
        format!("{block}_{kind}_{ordinal}")
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

fn unique_generated_id(
    occupied: &mut BTreeSet<String>,
    path: &str,
    missing: &MissingIdSummary,
    stem: &str,
) -> String {
    for salt in 0_u32.. {
        let hash = stable_hash(path, missing, stem, salt);
        for suffix_len in 4..=6 {
            let candidate = format!("{stem}_{}", base32_suffix(hash, suffix_len));
            if occupied.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
    unreachable!("unbounded salt search must find an unused generated ID")
}

fn stable_hash(path: &str, missing: &MissingIdSummary, stem: &str, salt: u32) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path
        .as_bytes()
        .iter()
        .chain([0].iter())
        .chain(stem.as_bytes())
        .chain([0].iter())
        .chain(kind_name(missing.kind).as_bytes())
        .chain([0].iter())
        .chain(missing.span.start.line().to_le_bytes().iter())
        .chain(missing.span.start.column().to_le_bytes().iter())
        .chain(salt.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn kind_name(kind: MissingIdKind) -> &'static str {
    match kind {
        MissingIdKind::Line => "line",
        MissingIdKind::Choice => "choice",
    }
}

fn base32_suffix(mut value: u64, len: usize) -> String {
    let mut suffix = vec!['A'; len];
    for slot in suffix.iter_mut().rev() {
        *slot = BASE32[(value & 31) as usize] as char;
        value >>= 5;
    }
    suffix.into_iter().collect()
}

fn insertion_text(text: &str, position: SourcePosition, generated_id: &str) -> String {
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

fn workspace_edit(uri: Uri, version: Option<i32>, edits: Vec<TextEdit>) -> WorkspaceEdit {
    WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier { uri, version },
            edits: edits.into_iter().map(OneOf::Left).collect(),
        }])),
        change_annotations: None,
    }
}

fn ranges_intersect(left: Range, right: Range) -> bool {
    position_le(left.start, right.end) && position_le(right.start, left.end)
}

fn position_le(left: lsp_types::Position, right: lsp_types::Position) -> bool {
    left.line < right.line || (left.line == right.line && left.character <= right.character)
}
