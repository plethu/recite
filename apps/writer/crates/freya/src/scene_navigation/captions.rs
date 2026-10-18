//! Caption identity depends on beat names and badges, not prose or edge labels.
use recite_writer_model::{ScriptBlock, scene_endings};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub(super) struct Caption {
    pub id: String,
    pub label: String,
    pub folded: String,
    is_default: bool,
    ending: bool,
}

pub(super) fn refresh(blocks: &[ScriptBlock], previous: Option<&Arc<[Caption]>>) -> Arc<[Caption]> {
    let endings = scene_endings(blocks);
    if let Some(previous) = previous.filter(|old| {
        old.len() == blocks.len()
            && old.iter().zip(blocks).all(|(caption, block)| {
                caption.id == block.id
                    && caption.is_default == block.is_default
                    && caption.ending == endings.contains(block.id.as_str())
            })
    }) {
        return previous.clone();
    }
    blocks
        .iter()
        .map(|block| {
            let ending = endings.contains(block.id.as_str());
            let label = format!(
                "{}{}{}",
                if block.is_default { "Start · " } else { "" },
                crate::palette::display_name(&block.id),
                if ending { " · end" } else { "" }
            );
            Caption {
                id: block.id.clone(),
                folded: label.to_lowercase(),
                label,
                is_default: block.is_default,
                ending,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
