//! One prepared substring index for the open catalogue queue.
use super::super::{catalogue::Catalogue, navigation::Destination, status::TranslationStatus};
use recite_core::po::PoEntryId;
use recite_writer_model::Passage;
use std::collections::BTreeMap;

struct IndexedEntry {
    id: PoEntryId,
    source: String,
    context: Option<String>,
    caption: Option<String>,
    forms: Vec<String>,
}
#[derive(Default)]
pub(in crate::localisation) struct QueueIndex {
    entries: Vec<Option<IndexedEntry>>,
}
impl QueueIndex {
    pub(in crate::localisation) fn new(catalogue: &Catalogue, passages: &[Passage]) -> Self {
        let mut current = BTreeMap::new();
        for passage in passages {
            current
                .entry((passage.id.as_str(), passage.text.as_str()))
                .or_insert(passage);
        }
        let entries = catalogue
            .document
            .entries()
            .iter()
            .map(|entry| {
                if entry.is_header() || entry.is_obsolete() {
                    return None;
                }

                let passage = entry
                    .context()
                    .and_then(|context| context.split('&').next())
                    .and_then(|anchor| current.get(&(anchor, entry.source_text())).copied());
                Some(IndexedEntry {
                    id: entry.id(),
                    source: entry.source_text().to_lowercase(),
                    context: entry.context().map(str::to_lowercase),
                    caption: Destination::with_current_passage(entry, passage)
                        .map(|d| d.caption().to_lowercase()),
                    forms: entry
                        .translation()
                        .into_iter()
                        .chain(entry.plural_translations().iter().map(|arm| arm.text()))
                        .map(str::to_lowercase)
                        .collect(),
                })
            })
            .collect();
        Self { entries }
    }
    pub(in crate::localisation) fn replace_forms(
        &mut self,
        updates: &mut [(PoEntryId, Vec<String>)],
    ) {
        for (id, forms) in updates {
            if let Some(entry) = self.entries.get_mut(id.index()).and_then(Option::as_mut) {
                std::mem::swap(&mut entry.forms, forms);
            }
        }
    }
    pub(in crate::localisation) fn matching_entries(
        &self,
        catalogue: &Catalogue,
        query: &str,
        only_attention: bool,
    ) -> Vec<PoEntryId> {
        let caption_query = query.replace('_', " ");
        self.entries
            .iter()
            .flatten()
            .filter(|entry| {
                (!only_attention
                    || TranslationStatus::for_entry(catalogue, entry.id).needs_attention())
                    && (query.is_empty()
                        || entry.source.contains(query)
                        || entry.context.as_ref().is_some_and(|id| id.contains(query))
                        || entry
                            .caption
                            .as_ref()
                            .is_some_and(|caption| caption.contains(&caption_query))
                        || if catalogue.changed(entry.id) {
                            catalogue
                                .forms(entry.id)
                                .any(|text| text.to_lowercase().contains(query))
                        } else {
                            entry.forms.iter().any(|text| text.contains(query))
                        })
            })
            .map(|entry| entry.id)
            .collect()
    }
}

#[cfg(test)]
mod tests;
