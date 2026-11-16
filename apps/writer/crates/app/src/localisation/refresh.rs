use super::*;
use recite_writer_session::localisation::{
    catalogue::RefreshPreview,
    refresh::changes::{Change, Kind},
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// A reviewed source refresh: one row per changed entry, each accepted unless kept, its diff
/// open under it on request.
pub struct Refresh {
    pub preview: Arc<RefreshPreview>,
    pub rows: Vec<Update>,
    /// The contexts of the entries kept as they are when the updates are applied.
    pub kept: BTreeSet<String>,
    /// The rows whose diff is open under them.
    pub open: BTreeSet<usize>,
}
/// One changed entry: what changed, where, and what a translator keeps with it.
pub struct Update {
    pub source: ComparisonRow,
    pub detail: Option<String>,
    pub context: String,
}
impl Refresh {
    pub fn new(preview: RefreshPreview) -> Self {
        let rows = preview.changes.iter().map(update).collect();
        Self {
            preview: Arc::new(preview),
            rows,
            kept: BTreeSet::new(),
            open: BTreeSet::new(),
        }
    }
    /// Whether the entry in `row` keeps its record rather than taking the update.
    pub fn is_kept(&self, row: usize) -> bool {
        self.rows
            .get(row)
            .is_some_and(|update| self.kept.contains(&update.context))
    }
}
/// The most of one text a row's diff shows; the PO file keeps all of it.
const SHOWN_CHARS: usize = 1_000;
fn bounded(text: &str) -> String {
    let mut shown: String = text.chars().take(SHOWN_CHARS).collect();
    if shown.len() < text.len() {
        shown.push('…');
    }
    shown
}
fn update(change: &Change) -> Update {
    let kind = match change.kind {
        Kind::Added => "New",
        Kind::Changed => "Changed",
        Kind::Removed => "Removed",
    };
    let caption: String = change.caption.chars().take(160).collect();
    let mut details = Vec::new();
    if !change.translation.is_empty() {
        details.push(format!("Retained translation\n{}", change.translation));
    }
    if !change.notes.is_empty() {
        details.push(format!("Translator notes\n{}", change.notes.join("\n")));
    }
    if !change.nearby.is_empty() {
        details.push(format!("Nearby source\n{}", change.nearby.join("\n")));
    }
    Update {
        source: ComparisonRow {
            caption: format!("{kind} · {caption}"),
            before: change.old.as_deref().map(bounded),
            after: change.new.as_deref().map(bounded),
        },
        detail: (!details.is_empty()).then(|| bounded(&details.join("\n\n"))),
        context: change.context.clone(),
    }
}
impl Writer {
    pub fn review_catalogue_sources(&mut self) {
        self.source_catalogue_action(work::SourceAction::Review);
    }
    pub fn apply_catalogue_sources(&mut self) {
        if let Some(refresh) = &self.localisation.refresh {
            self.source_catalogue_action(work::SourceAction::Apply(
                refresh.preview.clone(),
                refresh.kept.clone(),
            ));
        }
    }
    /// Accept the update in `row` (the default) or keep its entry as it is.
    pub fn keep_source_update(&mut self, row: usize, keep: bool) {
        if let Some(refresh) = &mut self.localisation.refresh
            && let Some(update) = refresh.rows.get(row)
        {
            let context = update.context.clone();
            if keep {
                refresh.kept.insert(context);
            } else {
                refresh.kept.remove(&context);
            }
        }
    }
    /// Open or close the diff under the update in `row`.
    pub fn toggle_source_update(&mut self, row: usize) {
        if let Some(refresh) = &mut self.localisation.refresh
            && !refresh.open.remove(&row)
        {
            refresh.open.insert(row);
        }
    }
}
