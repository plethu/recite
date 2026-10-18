//! Snapshot-owned manuscript metadata does not rescan a large beat on parent redraws.
use crate::design::tokens as t;
use freya::prelude::*;
use recite_writer_model::ScriptBlock;
use std::sync::Weak;

#[derive(Clone)]
pub(super) struct Slug {
    pub key: String,
    pub blocks: Weak<[ScriptBlock]>,
    pub index: usize,
}
impl PartialEq for Slug {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.blocks, &other.blocks)
            && self.index == other.index
            && self.key == other.key
    }
}
impl Component for Slug {
    fn render(&self) -> impl IntoElement {
        self.blocks
            .upgrade()
            .and_then(|blocks| blocks.get(self.index).map(|block| render(&self.key, block)))
            .unwrap_or_else(|| rect().into_element())
    }
}

/// The mono slug line under a beat title: file, block, flags, reply count.
fn render(key: &str, block: &ScriptBlock) -> Element {
    use t::Slug::{Flag, Meta};
    let file = if key.ends_with(".recite") {
        key.to_owned()
    } else {
        format!("{key}.recite")
    };
    let replies = block
        .entries
        .iter()
        .inspect(|_| {
            #[cfg(test)]
            tests::ENTRY_VISITS.with(|visits| visits.set(visits.get() + 1));
        })
        .filter(|entry| {
            matches!(entry, recite_writer_model::ScriptEntry::Passage(p)
                if matches!(p.kind, recite_writer_model::PassageKind::Choice { .. }))
        })
        .count();
    let mut parts = vec![Meta(file), Meta(format!(":: {}", block.id))];
    if block.is_default {
        parts.push(Flag("default".into()));
    }
    parts.push(Meta(match replies {
        0 => "no replies".into(),
        1 => "1 reply".into(),
        n => format!("{n} replies"),
    }));
    rect()
        .padding((0., 0., t::SPACE_LG, t::SPACE_XS))
        .child(t::slug(parts))
        .into_element()
}

#[cfg(test)]
mod tests;
