//! Source-ordered word index for project passage navigation; never executes dialogue.
use recite_compiler::SavedDocument;
use recite_core::Statement;
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
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}
impl DocumentIndex {
    pub fn build(documents: &[SavedDocument]) -> Self {
        let mut index = Self::default();
        for document in documents {
            let parsed =
                recite_parser::parse(document.key().as_str(), document.text()).lower_source_file();
            for block in &parsed.source_file.blocks {
                for root in &block.statements {
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
                        let id = index.hits.len();
                        let hit = SearchHit {
                            document: document.key().as_str().into(),
                            beat: block.id.to_string(),
                            speaker,
                            text: text.clone(),
                        };
                        for word in words(&format!(
                            "{} {} {} {}",
                            hit.document, hit.beat, hit.speaker, hit.text
                        )) {
                            index.words.entry(word).or_default().push(id);
                        }
                        index.hits.push(hit);
                    });
                }
            }
        }
        index
    }
    /// All query words must match. Results preserve document/source order.
    pub fn search(&self, query: &str, limit: usize) -> (usize, Vec<SearchHit>) {
        let terms = words(query);
        if terms.is_empty() {
            return (0, Vec::new());
        }
        let Some(mut postings) = terms
            .iter()
            .map(|word| self.words.get(word))
            .collect::<Option<Vec<_>>>()
        else {
            return (0, Vec::new());
        };
        postings.sort_by_key(|items| items.len());
        let mut total = 0;
        let mut hits = Vec::new();
        for &id in postings[0] {
            if postings[1..]
                .iter()
                .all(|items| items.binary_search(&id).is_ok())
            {
                total += 1;
                if hits.len() < limit {
                    hits.push(self.hits[id].clone());
                }
            }
        }
        (total, hits)
    }
}

/// Per-document immutable shards let saves replace only the changed index.
#[derive(Clone, Default)]
pub struct SearchIndex {
    documents: Vec<(String, std::sync::Arc<DocumentIndex>)>,
}
impl SearchIndex {
    pub fn build(documents: &[SavedDocument]) -> Self {
        Self {
            documents: documents
                .iter()
                .map(|doc| {
                    (
                        doc.key().as_str().to_owned(),
                        std::sync::Arc::new(DocumentIndex::build(std::slice::from_ref(doc))),
                    )
                })
                .collect(),
        }
    }
    pub fn replace_document(&mut self, document: SavedDocument) {
        let next = std::sync::Arc::new(DocumentIndex::build(std::slice::from_ref(&document)));
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
        let mut count = 0;
        let mut hits = Vec::new();
        for (_, document) in &self.documents {
            let (found, matches) = document.search(query, limit.saturating_sub(hits.len()));
            count += found;
            hits.extend(matches);
        }
        (count, hits)
    }
}
