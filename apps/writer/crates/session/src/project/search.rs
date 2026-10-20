//! Search indexes describe saved sources, independently of schema and overlays.
use super::ProjectFiles;
use recite_compiler::authoring::SavedDocument;
use recite_writer_model::SearchIndex;
use std::sync::Arc;

impl ProjectFiles {
    pub(super) fn refreshed_search(&self, documents: &[SavedDocument]) -> Arc<SearchIndex> {
        let previous = &self.context.documents;
        if documents.len() != previous.len()
            || documents
                .iter()
                .zip(previous)
                .any(|(next, old)| next.key() != old.key())
        {
            return Arc::new(SearchIndex::build(documents));
        }
        let mut changed = None;
        for (next, old) in documents.iter().zip(previous) {
            if next != old {
                changed
                    .get_or_insert_with(|| self.search.as_ref().clone())
                    .replace_document(next.clone());
            }
        }
        changed
            .map(Arc::new)
            .unwrap_or_else(|| Arc::clone(&self.search))
    }
}

#[cfg(test)]
mod tests;
