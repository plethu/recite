use super::{SearchHit, words};
use recite_compiler::authoring::SavedDocument;
use recite_core::ast::Statement;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[derive(Default)]
pub(super) struct RegionIndex {
    pub(super) hits: Vec<SearchHit>,
    pub(super) words: BTreeMap<String, Vec<usize>>,
}
impl RegionIndex {
    pub(super) fn build<E>(
        document: &SavedDocument,
        regions: &[recite_parser::SourceRegion<'_>],
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Self, E> {
        checkpoint()?;
        let mut index = Self::default();
        for region in regions {
            checkpoint()?;
            let mut context = None;
            region.try_visit_statements(document.key().as_str(), |block, root| {
                checkpoint()?;
                let current = context.get_or_insert_with(|| SearchContext::new(document, block));
                // A parser restart region may contain indented recovered blocks.
                if current.beat.as_ref() != block.id.as_str() || current.block_span != block.span {
                    *current = SearchContext::new(document, block);
                }
                index.add_root(current, root);
                Ok(())
            })?;
            checkpoint()?;
        }
        // Published shards are immutable; release geometric builder capacity.
        index.hits.shrink_to_fit();
        for postings in index.words.values_mut() {
            checkpoint()?;
            postings.shrink_to_fit();
        }
        Ok(index)
    }

    pub(super) fn add_root(&mut self, context: &mut SearchContext, root: &Statement) {
        root.visit_depth_first(&mut |statement| {
            let (text, speaker) = match statement {
                Statement::Line(line) => (
                    &line.source_text.text,
                    context.speaker(line.speaker.as_ref()),
                ),
                Statement::Choice(choice) => (&choice.source_text.text, Arc::default()),
                _ => return,
            };
            let id = self.hits.len();
            let hit = SearchHit {
                document: context.document.clone(),
                beat: context.beat.clone(),
                speaker,
                text: text.clone(),
            };
            for field in [
                hit.document.as_ref(),
                hit.beat.as_ref(),
                hit.speaker.as_ref(),
                hit.text.as_str(),
            ] {
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
    pub(super) fn search<'a>(
        &'a self,
        terms: &BTreeSet<String>,
        postings: &mut Vec<&'a [usize]>,
        limit: usize,
        mut push: impl FnMut(usize),
    ) -> usize {
        postings.clear();
        for word in terms {
            let Some(items) = self.words.get(word) else {
                return 0;
            };
            postings.push(items);
        }
        postings.sort_by_key(|items| items.len());
        let Some(shortest) = postings.first().copied() else {
            return 0;
        };
        if postings.len() == 1 {
            for &id in shortest.iter().take(limit) {
                push(id);
            }
            return shortest.len();
        }
        let mut total = 0;
        for &id in shortest {
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

pub(super) struct SearchContext {
    document: Arc<str>,
    beat: Arc<str>,
    speaker: Arc<str>,
    last_speaker: Arc<str>,
    block_span: recite_core::SourceSpan,
}
impl SearchContext {
    pub(super) fn new(document: &SavedDocument, block: &recite_core::ast::Block) -> Self {
        let speaker: Arc<str> = block
            .default_speaker
            .as_ref()
            .map_or_else(Arc::default, |s| Arc::from(s.as_str()));
        Self {
            document: Arc::from(document.key().as_str()),
            beat: Arc::from(block.id.as_str()),
            last_speaker: speaker.clone(),
            speaker,
            block_span: block.span.clone(),
        }
    }
    fn speaker(&mut self, explicit: Option<&recite_core::SpeakerId>) -> Arc<str> {
        let Some(speaker) = explicit else {
            return self.speaker.clone();
        };
        // Consecutive explicit labels share storage without an interning table.
        if self.last_speaker.as_ref() != speaker.as_str() {
            self.last_speaker = Arc::from(speaker.as_str());
        }
        self.last_speaker.clone()
    }
}
