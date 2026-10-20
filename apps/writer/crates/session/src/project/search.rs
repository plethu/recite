//! Search indexes describe saved sources, independently of schema and overlays.
use super::FileError;
use recite_compiler::authoring::{SavedDocument, WorkControl};
use recite_writer_model::SearchIndex;
use std::sync::Arc;

pub(super) fn refresh_search(
    previous: &[SavedDocument],
    search: &Arc<SearchIndex>,
    documents: &[SavedDocument],
    control: &dyn WorkControl,
) -> Result<Arc<SearchIndex>, FileError> {
    control.checkpoint()?;
    if documents.len() != previous.len()
        || documents
            .iter()
            .zip(previous)
            .any(|(next, old)| next.key() != old.key())
    {
        return Ok(Arc::new(SearchIndex::build_with_control(
            documents, control,
        )?));
    }
    let mut changed = None;
    for (next, old) in documents.iter().zip(previous) {
        control.checkpoint()?;
        if next != old {
            changed
                .get_or_insert_with(|| search.as_ref().clone())
                .replace_document(next.clone());
        }
    }
    Ok(changed.map(Arc::new).unwrap_or_else(|| Arc::clone(search)))
}

#[cfg(test)]
mod tests;
