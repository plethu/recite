//! Recording items, their derived state and handoff rows (spec §9.9).
//!
//! Items join compiled lines with the recording language's reviewed translations; the sidecar
//! codec and revision live in `recite_core::recording`. Callers own all file I/O.

mod import;

use std::collections::BTreeSet;

use recite_core::compiled::{CompiledDialogue, CompiledStatementKind, StatementRange};
use recite_core::po::{PoDocument, PoReviewState};
use recite_core::recording::{HandoffRow, RecordingRevision, RecordingSidecar, StoredStatus};
use recite_core::{ScalarValue, Value};

pub use import::{ImportPlan, ImportRefusal, RowRefusal, plan_import};

/// One compiled line in one recording language.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingItem {
    pub id: String,
    pub block: String,
    pub file: String,
    pub line: u32,
    pub speaker: Option<String>,
    /// Present direction keys and their values, in declared key order.
    pub direction: Vec<(String, String)>,
    pub source_forms: Vec<String>,
    /// The forms to record; `None` while a locale item has no reviewed translation.
    pub forms: Option<Vec<String>>,
    pub revision: Option<RecordingRevision>,
}

/// Every compiled line as a recording item, in block and statement order.
///
/// Without a catalogue the language is the source. With one, an item is ready only when its
/// catalogue record matches the current source exactly and is reviewed.
#[must_use]
pub fn recording_items(
    dialogue: &CompiledDialogue,
    direction_keys: &[String],
    catalog: Option<&PoDocument>,
) -> Vec<RecordingItem> {
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for block in &dialogue.blocks {
        let mut lines = Vec::new();
        collect_lines(dialogue, block.statements, &mut lines);
        for index in lines {
            if !seen.insert(index) {
                continue;
            }
            let line = &dialogue.lines[index as usize];
            let speaker = line
                .speaker
                .or(block.default_speaker)
                .map(|speaker| dialogue.speakers[speaker.as_u32() as usize].id.to_string());
            let metadata = &dialogue.metadata[line.metadata.start.as_u32() as usize..]
                [..line.metadata.len as usize];
            let direction = direction_keys
                .iter()
                .filter_map(|key| {
                    let entry = metadata.iter().find(|entry| &entry.key == key)?;
                    Some((key.clone(), value_text(&entry.value)))
                })
                .collect::<Vec<_>>();
            let span = &dialogue.source_maps[line.source_map.as_u32() as usize].span;
            let source_forms = std::iter::once(line.source_text.clone())
                .chain(line.plural_source_text.clone())
                .collect::<Vec<_>>();
            let forms = match catalog {
                None => Some(source_forms.clone()),
                Some(catalog) => reviewed_forms(catalog, line.id.as_str(), &source_forms),
            };
            let revision = forms.as_ref().map(|forms| {
                let forms = forms.iter().map(String::as_str).collect::<Vec<_>>();
                RecordingRevision::of(&forms, speaker.as_deref(), &direction)
            });
            items.push(RecordingItem {
                id: line.id.to_string(),
                block: block.id.to_string(),
                file: span.file.clone(),
                line: span.start.line(),
                speaker,
                direction,
                source_forms,
                forms,
                revision,
            });
        }
    }
    items
}

fn collect_lines(dialogue: &CompiledDialogue, range: StatementRange, lines: &mut Vec<u32>) {
    let start = range.start.as_u32() as usize;
    for statement in &dialogue.statements[start..start + range.len as usize] {
        match &statement.kind {
            CompiledStatementKind::Line(line) => lines.push(line.as_u32()),
            CompiledStatementKind::Prompt {
                line: Some(line), ..
            } => lines.push(line.as_u32()),
            CompiledStatementKind::If {
                then_statements,
                else_statements,
                ..
            } => {
                collect_lines(dialogue, *then_statements, lines);
                collect_lines(dialogue, *else_statements, lines);
            }
            CompiledStatementKind::Match { arms, .. } => {
                let first = arms.start.as_u32() as usize;
                for arm in &dialogue.match_arms[first..first + arms.len as usize] {
                    collect_lines(dialogue, arm.statements, lines);
                }
            }
            _ => {}
        }
    }
}

fn value_text(value: &Value) -> String {
    fn scalar(value: &ScalarValue) -> String {
        match value {
            ScalarValue::String(text) => text.clone(),
            ScalarValue::Integer(number) => number.to_string(),
            ScalarValue::Float(number) => number.to_string(),
            ScalarValue::Boolean(flag) => flag.to_string(),
        }
    }
    match value {
        Value::Scalar(value) => scalar(value),
        Value::Array(values) => values.iter().map(scalar).collect::<Vec<_>>().join(", "),
    }
}

fn reviewed_forms(catalog: &PoDocument, id: &str, source: &[String]) -> Option<Vec<String>> {
    let entry = catalog
        .entries()
        .iter()
        .find(|entry| !entry.is_header() && !entry.is_obsolete() && entry.context() == Some(id))?;
    if entry.source_text() != source[0]
        || entry.plural_source_text() != source.get(1).map(String::as_str)
        || entry.review_state() != PoReviewState::Reviewed
    {
        return None;
    }
    Some(if entry.is_plural() {
        entry
            .plural_translations()
            .iter()
            .map(|form| form.text().to_owned())
            .collect()
    } else {
        vec![entry.translation()?.to_owned()]
    })
}

/// An item's state, derived from the current item and the stored sidecar entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RecordingState {
    NotReady,
    ToRecord,
    Recorded,
    Approved,
    Unvoiced,
    Stale,
    Obsolete,
}

impl RecordingState {
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotReady => "not_ready",
            Self::ToRecord => "to_record",
            Self::Recorded => "recorded",
            Self::Approved => "approved",
            Self::Unvoiced => "unvoiced",
            Self::Stale => "stale",
            Self::Obsolete => "obsolete",
        }
    }

    /// Recording work a person still owes; `not_ready` belongs to the catalogue.
    #[must_use]
    pub const fn needs_attention(self) -> bool {
        matches!(self, Self::ToRecord | Self::Stale | Self::Obsolete)
    }

    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        [
            Self::NotReady,
            Self::ToRecord,
            Self::Recorded,
            Self::Approved,
            Self::Unvoiced,
            Self::Stale,
            Self::Obsolete,
        ]
        .into_iter()
        .find(|state| state.key() == key)
    }
}

/// One report row: a current item, or an obsolete sidecar ID with no item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingEntry<'a> {
    pub id: String,
    pub state: RecordingState,
    pub item: Option<&'a RecordingItem>,
}

/// Items in source order, then obsolete sidecar IDs in ID order.
#[must_use]
pub fn recording_report<'a>(
    items: &'a [RecordingItem],
    sidecar: &RecordingSidecar,
) -> Vec<RecordingEntry<'a>> {
    let mut entries = items
        .iter()
        .map(|item| {
            let state = match (&item.revision, sidecar.items.get(&item.id)) {
                (_, Some(stored)) if stored.status == StoredStatus::Unvoiced => {
                    RecordingState::Unvoiced
                }
                (None, _) => RecordingState::NotReady,
                (Some(_), None) => RecordingState::ToRecord,
                (Some(current), Some(stored)) if stored.revision().as_ref() != Some(current) => {
                    RecordingState::Stale
                }
                (Some(_), Some(stored)) if stored.status == StoredStatus::Approved => {
                    RecordingState::Approved
                }
                (Some(_), Some(_)) => RecordingState::Recorded,
            };
            RecordingEntry {
                id: item.id.clone(),
                state,
                item: Some(item),
            }
        })
        .collect::<Vec<_>>();
    let current = items
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    entries.extend(
        sidecar
            .items
            .keys()
            .filter(|id| !current.contains(id.as_str()))
            .map(|id| RecordingEntry {
                id: id.clone(),
                state: RecordingState::Obsolete,
                item: None,
            }),
    );
    entries
}

/// Handoff rows for ready entries, in report order.
#[must_use]
pub fn handoff_rows(
    entries: &[RecordingEntry<'_>],
    locale: Option<&str>,
    handoff: &str,
) -> Vec<HandoffRow> {
    entries
        .iter()
        .filter_map(|entry| {
            let item = entry.item?;
            let revision = item.revision.as_ref()?;
            Some(HandoffRow {
                id: item.id.clone(),
                locale: locale.unwrap_or_default().to_owned(),
                handoff: handoff.to_owned(),
                block: item.block.clone(),
                file: item.file.clone(),
                line: item.line,
                speaker: item.speaker.clone().unwrap_or_default(),
                direction: item
                    .direction
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("; "),
                source_text: item.source_forms.join("\n"),
                text: item.forms.as_deref().unwrap_or_default().join("\n"),
                revision: revision.to_string(),
                state: entry.state.key().to_owned(),
                ..HandoffRow::default()
            })
        })
        .collect()
}
