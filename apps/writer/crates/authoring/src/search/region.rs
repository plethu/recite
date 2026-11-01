use super::{HitOrigin, SearchHit, words};
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
                if current.block.beat.as_ref() != block.id.as_str()
                    || current.block_span != block.span
                {
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
            let (text, origin, passage) = match statement {
                Statement::Line(line) => (
                    &line.source_text.text,
                    context.speaker(line.speaker.as_ref()),
                    line.source_id
                        .stable_id()
                        .filter(|_| line.plural_source_text.is_none()),
                ),
                Statement::Choice(choice) => (
                    &choice.source_text.text,
                    context.reply.clone(),
                    choice.source_id.stable_id(),
                ),
                _ => return,
            };
            let id = self.hits.len();
            let hit = SearchHit {
                origin,
                passage: passage.cloned(),
                text: text.as_str().into(),
            };
            for field in [hit.document(), hit.beat(), hit.speaker(), hit.text.as_ref()] {
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
    /// Lines without an explicit speaker use the block default.
    block: Arc<HitOrigin>,
    /// Replies have no speaker.
    reply: Arc<HitOrigin>,
    last_speaker: Arc<HitOrigin>,
    block_span: recite_core::SourceSpan,
}
impl SearchContext {
    pub(super) fn new(document: &SavedDocument, block: &recite_core::ast::Block) -> Self {
        let document: Arc<str> = Arc::from(document.key().as_str());
        let beat: Arc<str> = Arc::from(block.id.as_str());
        let origin = |speaker| {
            Arc::new(HitOrigin {
                document: document.clone(),
                beat: beat.clone(),
                speaker,
            })
        };
        let default = origin(
            block
                .default_speaker
                .as_ref()
                .map_or_else(Arc::default, |s| Arc::from(s.as_str())),
        );
        Self {
            reply: origin(Arc::default()),
            last_speaker: default.clone(),
            block: default,
            block_span: block.span.clone(),
        }
    }
    fn speaker(&mut self, explicit: Option<&recite_core::SpeakerId>) -> Arc<HitOrigin> {
        let Some(speaker) = explicit else {
            return self.block.clone();
        };
        // Consecutive explicit labels share storage without an interning table.
        if self.last_speaker.speaker.as_ref() != speaker.as_str() {
            self.last_speaker = Arc::new(HitOrigin {
                document: self.block.document.clone(),
                beat: self.block.beat.clone(),
                speaker: Arc::from(speaker.as_str()),
            });
        }
        self.last_speaker.clone()
    }
}
