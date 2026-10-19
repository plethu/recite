//! Reuse exact, grammar-bounded source shards without duplicating a map per block.
use super::region::RegionIndex;
use recite_compiler::authoring::{Interrupted, SavedDocument, WorkControl};
use recite_parser::SourceRegion;
use std::{convert::Infallible, ops::Range, sync::Arc};

// Amortize repeated vocabulary while bounding an ordinary block edit's rebuild.
const REGIONS_PER_SHARD: usize = 32;
pub(super) struct IndexedRegions {
    pub bytes: Range<usize>,
    pub count: usize,
    pub index: Arc<RegionIndex>,
}
pub(super) struct DocumentIndex {
    source: SavedDocument,
    pub shards: Vec<IndexedRegions>,
}
impl DocumentIndex {
    pub fn build(document: &SavedDocument, previous: Option<&Self>) -> Self {
        match Self::build_with_checkpoints(document, previous, &mut || Ok::<(), Infallible>(())) {
            Ok(index) => index,
            Err(impossible) => match impossible {},
        }
    }
    pub fn build_with_control(
        document: &SavedDocument,
        control: &dyn WorkControl,
    ) -> Result<Self, Interrupted> {
        Self::build_with_checkpoints(document, None, &mut || control.checkpoint())
    }
    fn build_with_checkpoints<E>(
        document: &SavedDocument,
        previous: Option<&Self>,
        checkpoint: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<Self, E> {
        checkpoint()?;
        let regions = recite_parser::source_regions(document.text());
        let mut start = 0;
        let mut end = regions.len();
        let mut shards = Vec::new();
        let mut suffix = Vec::new();
        if let Some(old) = previous.filter(|old| old.source.key() == document.key()) {
            for shard in &old.shards {
                checkpoint()?;
                if start + shard.count > end {
                    break;
                }
                let bytes = range(&regions[start..start + shard.count]);
                if document.text()[bytes.clone()] != old.source.text()[shard.bytes.clone()] {
                    break;
                }
                shards.push(IndexedRegions {
                    bytes,
                    count: shard.count,
                    index: shard.index.clone(),
                });
                start += shard.count;
            }
            for shard in old
                .shards
                .iter()
                .rev()
                .take(old.shards.len() - shards.len())
            {
                checkpoint()?;
                if shard.count > end - start {
                    break;
                }
                let bytes = range(&regions[end - shard.count..end]);
                if document.text()[bytes.clone()] != old.source.text()[shard.bytes.clone()] {
                    break;
                }
                suffix.push(IndexedRegions {
                    bytes,
                    count: shard.count,
                    index: shard.index.clone(),
                });
                end -= shard.count;
            }
        }
        // Keep rebuilt boundaries at least half full. Otherwise repeated inserts
        // could accumulate singleton maps between reused suffix shards.
        if end > start && end - start < REGIONS_PER_SHARD / 2 {
            if let Some(neighbor) = shards.pop() {
                start -= neighbor.count;
            } else if let Some(neighbor) = suffix.pop() {
                end += neighbor.count;
            }
        }
        let count = end - start;
        let groups = count.div_ceil(REGIONS_PER_SHARD);
        let mut first = start;
        for group in 0..groups {
            let size = count / groups + usize::from(group < count % groups);
            let regions = &regions[first..first + size];
            checkpoint()?;
            let index = RegionIndex::build(document, regions, checkpoint)?;
            shards.push(IndexedRegions {
                bytes: range(regions),
                count: regions.len(),
                index: Arc::new(index),
            });
            first += size;
        }
        shards.extend(suffix.into_iter().rev());
        checkpoint()?;
        Ok(Self {
            source: document.clone(),
            shards,
        })
    }
}
fn range(regions: &[SourceRegion<'_>]) -> Range<usize> {
    regions[0].byte_range().start..regions[regions.len() - 1].byte_range().end
}
