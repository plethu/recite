//! Source-ordered word index for project passage navigation; never executes dialogue.
mod results;
use recite_compiler::authoring::{Interrupted, SavedDocument, WorkControl};
mod region;
use region::RegionIndex;
pub use results::SearchResults;
mod document;
use document::DocumentIndex;
use std::sync::Arc;

/// Immutable context labels share storage across hits; prose belongs to this saved snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub document: Arc<str>,
    pub beat: Arc<str>,
    pub speaker: Arc<str>,
    pub text: String,
}
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}
/// Per-document immutable shards let saves replace only the changed index.
#[derive(Clone, Default)]
pub struct SearchIndex {
    documents: Vec<(String, Arc<DocumentIndex>)>,
}
impl SearchIndex {
    pub fn build(documents: &[SavedDocument]) -> Self {
        let mut index = Self {
            documents: Vec::with_capacity(documents.len()),
        };
        for document in documents {
            index.add_document(document);
        }
        index
    }

    /// Check parse and indexing boundaries; publish only a complete index.
    pub fn build_with_control(
        documents: &[SavedDocument],
        control: &dyn WorkControl,
    ) -> Result<Self, Interrupted> {
        control.checkpoint()?;
        let mut index = Self {
            documents: Vec::with_capacity(documents.len()),
        };
        for document in documents {
            control.checkpoint()?;
            let shard = DocumentIndex::build_with_control(document, control)?;
            index
                .documents
                .push((document.key().as_str().to_owned(), Arc::new(shard)));
            control.checkpoint()?;
        }
        Ok(index)
    }

    fn add_document(&mut self, document: &SavedDocument) {
        self.documents.push((
            document.key().as_str().to_owned(),
            Arc::new(DocumentIndex::build(document, None)),
        ));
    }
    pub fn replace_document(&mut self, document: SavedDocument) {
        if let Some((_, index)) = self
            .documents
            .iter_mut()
            .find(|(key, _)| key == document.key().as_str())
        {
            *index = Arc::new(DocumentIndex::build(&document, Some(index)));
        } else {
            self.add_document(&document);
        }
    }
    pub fn len(&self) -> usize {
        self.documents
            .iter()
            .flat_map(|(_, d)| &d.shards)
            .map(|r| r.index.hits.len())
            .sum()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn search(&self, query: &str, limit: usize) -> (usize, Vec<SearchHit>) {
        self.search_with(query, limit, |document, id| document.hits[id].clone())
    }
    /// Immutable result handles retain their saved index shards, including across saves.
    pub fn search_results(&self, query: &str, limit: usize) -> SearchResults {
        let (total, hits) = self.search_with(query, limit, |document, id| results::SearchMatch {
            document: document.clone(),
            id,
        });
        SearchResults::new(total, hits)
    }
    fn search_with<T>(
        &self,
        query: &str,
        limit: usize,
        mut hit: impl FnMut(&Arc<RegionIndex>, usize) -> T,
    ) -> (usize, Vec<T>) {
        let terms = words(query).collect::<std::collections::BTreeSet<_>>();
        // Reuse one query workspace across all immutable shards.
        let mut postings = Vec::with_capacity(terms.len());
        let mut count = 0;
        let mut hits = Vec::new();
        for (_, document) in &self.documents {
            for shard in &document.shards {
                let region = &shard.index;
                let remaining = limit.saturating_sub(hits.len());
                count += region.search(&terms, &mut postings, remaining, |id| {
                    hits.push(hit(region, id))
                });
            }
        }
        (count, hits)
    }
}

#[cfg(test)]
mod tests;
