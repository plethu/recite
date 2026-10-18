//! Caption identity depends on beat names and badges, not prose or edge labels.
use recite_compiler::authoring::{AuthoringSummary, BlockDefinitionSummary};
use std::sync::Arc;

// Summary value equality can walk metadata containing floats. Memo invalidation
// needs only ownership identity, so Source drafts do not rescan accepted facts.
#[derive(Clone)]
pub(super) struct SummarySnapshot(pub Option<Arc<AuthoringSummary>>);
impl PartialEq for SummarySnapshot {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Caption {
    pub id: String,
    pub label: String,
    pub folded: String,
    is_default: bool,
    ending: bool,
}

pub(super) fn refresh(
    blocks: &[BlockDefinitionSummary],
    previous: Option<&Arc<[Caption]>>,
) -> Arc<[Caption]> {
    // Duplicate authored names share badges, as they do in Script navigation.
    let endings: std::collections::BTreeSet<_> = blocks
        .iter()
        .filter(|block| block.has_end_destination())
        .map(|block| block.id().as_str())
        .collect();
    if let Some(previous) = previous.filter(|old| {
        old.len() == blocks.len()
            && old.iter().zip(blocks).all(|(caption, block)| {
                caption.id == block.id().as_str()
                    && caption.is_default == block.is_default()
                    && caption.ending == endings.contains(block.id().as_str())
            })
    }) {
        return previous.clone();
    }
    blocks
        .iter()
        .map(|block| {
            let ending = endings.contains(block.id().as_str());
            let label = format!(
                "{}{}{}",
                if block.is_default() { "Start · " } else { "" },
                crate::palette::display_name(block.id().as_str()),
                if ending { " · end" } else { "" }
            );
            Caption {
                id: block.id().to_string(),
                folded: label.to_lowercase(),
                label,
                is_default: block.is_default(),
                ending,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
