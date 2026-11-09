use super::*;
use recite_writer_session::localisation::{
    catalogue::RefreshPreview,
    refresh::changes::{Change, Kind},
};
use std::sync::Arc;

pub struct Refresh {
    pub preview: Arc<RefreshPreview>,
    pub pages: Vec<Page>,
    pub page: usize,
}
pub struct Page {
    pub source: ComparisonRow,
    pub detail: Option<String>,
    pub(crate) change: usize,
}
impl Refresh {
    pub fn new(preview: RefreshPreview) -> Self {
        let pages = preview
            .changes
            .iter()
            .enumerate()
            .flat_map(|(index, change)| change_pages(change, index))
            .collect();
        Self {
            preview: Arc::new(preview),
            pages,
            page: 0,
        }
    }
}
fn change_pages(change: &Change, change_index: usize) -> Vec<Page> {
    let kind = match change.kind {
        Kind::Added => "New",
        Kind::Changed => "Changed",
        Kind::Removed => "Removed",
    };
    let caption: String = change.caption.chars().take(160).collect();
    let before = change.old.as_deref().map(text::pages).unwrap_or_default();
    let after = change.new.as_deref().map(text::pages).unwrap_or_default();
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
    let detail = text::pages(&details.join("\n\n"));
    (0..before.len().max(after.len()).max(detail.len()))
        .map(|index| Page {
            change: change_index,
            source: ComparisonRow {
                caption: format!("{kind} · {caption}"),
                before: before.get(index).cloned(),
                after: after.get(index).cloned(),
            },
            detail: detail.get(index).cloned(),
        })
        .collect()
}
impl Writer {
    pub fn review_catalogue_sources(&mut self) {
        self.source_catalogue_action(work::SourceAction::Review);
    }
    pub fn apply_catalogue_sources(&mut self) {
        if let Some(refresh) = &self.localisation.refresh {
            self.source_catalogue_action(work::SourceAction::Apply(refresh.preview.clone()));
        }
    }
}
