//! Source-ordered word index for project passage navigation; never executes dialogue.
mod results;
use recite_compiler::authoring::{Interrupted, SavedDocument, WorkControl};
use recite_core::ast::Statement;
pub use results::SearchResults;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub document: String,
    pub beat: String,
    pub speaker: String,
    pub text: String,
}
#[derive(Default)]
struct DocumentIndex {
    hits: Vec<SearchHit>,
    words: BTreeMap<String, Vec<usize>>,
}
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}
impl DocumentIndex {
    fn build(document: &SavedDocument) -> Self {
        let lowered = recite_parser::lower_source_file(document.key().as_str(), document.text());
        let mut index = Self::default();
        for block in &lowered.source_file.blocks {
            for root in &block.statements {
                index.add_root(document, block, root);
            }
        }
        index
    }

    fn build_with_control(
        document: &SavedDocument,
        control: &dyn WorkControl,
    ) -> Result<Self, Interrupted> {
        control.checkpoint()?;
        let parsed = recite_parser::parse(document.key().as_str(), document.text());
        control.checkpoint()?;
        let lowered = parsed.lower_source_file();
        control.checkpoint()?;
        let mut index = Self::default();
        for block in &lowered.source_file.blocks {
            control.checkpoint()?;
            for root in &block.statements {
                control.checkpoint()?;
                index.add_root(document, block, root);
            }
        }
        control.checkpoint()?;
        Ok(index)
    }

    fn add_root(
        &mut self,
        document: &SavedDocument,
        block: &recite_core::ast::Block,
        root: &Statement,
    ) {
        root.visit_depth_first(&mut |statement| {
            let (text, speaker) = match statement {
                Statement::Line(line) => (
                    &line.source_text.text,
                    line.speaker
                        .as_ref()
                        .or(block.default_speaker.as_ref())
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                ),
                Statement::Choice(choice) => (&choice.source_text.text, String::new()),
                _ => return,
            };
            let id = self.hits.len();
            let hit = SearchHit {
                document: document.key().as_str().into(),
                beat: block.id.to_string(),
                speaker,
                text: text.clone(),
            };
            for field in [&hit.document, &hit.beat, &hit.speaker, &hit.text] {
                for word in words(field) {
                    let postings = self.words.entry(word).or_default();
                    // Hits arrive in source order, so the last ID deduplicates
                    // repeated words within and across this hit's fields.
                    if postings.last() != Some(&id) {
                        postings.push(id);
                    }
                }
            }
            self.hits.push(hit);
        });
    }
    /// All query words must match. Results preserve document/source order.
    fn search(&self, terms: &BTreeSet<String>, limit: usize, mut push: impl FnMut(usize)) -> usize {
        if terms.is_empty() {
            return 0;
        }
        let Some(mut postings) = terms
            .iter()
            .map(|word| self.words.get(word))
            .collect::<Option<Vec<_>>>()
        else {
            return 0;
        };
        if postings.len() == 1 {
            for &id in postings[0].iter().take(limit) {
                push(id);
            }
            return postings[0].len();
        }
        postings.sort_by_key(|items| items.len());
        let mut total = 0;
        for &id in postings[0] {
            if postings[1..]
                .iter()
                .all(|items| items.binary_search(&id).is_ok())
            {
                total += 1;
                if total <= limit {
                    push(id);
                }
            }
        }
        total
    }
}

/// Per-document immutable shards let saves replace only the changed index.
#[derive(Clone, Default)]
pub struct SearchIndex {
    documents: Vec<(String, std::sync::Arc<DocumentIndex>)>,
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
            index.documents.push((
                document.key().as_str().to_owned(),
                std::sync::Arc::new(shard),
            ));
            control.checkpoint()?;
        }
        Ok(index)
    }

    fn add_document(&mut self, document: &SavedDocument) {
        self.documents.push((
            document.key().as_str().to_owned(),
            std::sync::Arc::new(DocumentIndex::build(document)),
        ));
    }
    pub fn replace_document(&mut self, document: SavedDocument) {
        let next = std::sync::Arc::new(DocumentIndex::build(&document));
        if let Some((_, index)) = self
            .documents
            .iter_mut()
            .find(|(key, _)| key == document.key().as_str())
        {
            *index = next;
        } else {
            self.documents.push((document.key().as_str().into(), next));
        }
    }
    pub fn len(&self) -> usize {
        self.documents.iter().map(|(_, d)| d.hits.len()).sum()
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
        mut hit: impl FnMut(&std::sync::Arc<DocumentIndex>, usize) -> T,
    ) -> (usize, Vec<T>) {
        let terms = words(query).collect();
        let mut count = 0;
        let mut hits = Vec::new();
        for (_, document) in &self.documents {
            let remaining = limit.saturating_sub(hits.len());
            count += document.search(&terms, remaining, |id| hits.push(hit(document, id)));
        }
        (count, hits)
    }
}
