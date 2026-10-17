//! Search indexes describe saved sources, independently of schema and overlays.
use super::ProjectFiles;
use recite_compiler::authoring::SavedDocument;
use recite_writer_model::SearchIndex;
use std::sync::Arc;

impl ProjectFiles {
    pub(super) fn refreshed_search(&self, documents: &[SavedDocument]) -> Arc<SearchIndex> {
        if documents == self.context.documents {
            Arc::clone(&self.search)
        } else {
            Arc::new(SearchIndex::build(documents))
        }
    }
}

#[cfg(test)]
mod tests;
