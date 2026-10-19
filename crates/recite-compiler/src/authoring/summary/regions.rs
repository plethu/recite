//! Compact offsets into a document summary; cached regions do not duplicate it.
use super::AuthoringSummary;
use crate::region_outputs;
use crate::region_outputs::region_mut;
use std::{ops::Range, sync::Arc};

pub(crate) type SummaryRanges = [Range<usize>; 7];

impl AuthoringSummary {
    pub(crate) fn region_ranges(&self) -> SummaryRanges {
        [
            0..self.blocks.len(),
            0..self.block_references.len(),
            0..self.stable_ids.len(),
            0..self.metadata.len(),
            0..self.condition_functions.len(),
            0..self.effect_functions.len(),
            0..self.bindings.len(),
        ]
    }

    pub(crate) fn join_regions(
        parts: &[(&Self, &SummaryRanges)],
        previous: Option<&Arc<Self>>,
    ) -> Arc<Self> {
        let joined = Self {
            blocks: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
                previous.map(|old| &old.blocks),
            ),
            block_references: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.block_references[ranges[1].clone()]),
                previous.map(|old| &old.block_references),
            ),
            stable_ids: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.stable_ids[ranges[2].clone()]),
                previous.map(|old| &old.stable_ids),
            ),
            metadata: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.metadata[ranges[3].clone()]),
                previous.map(|old| &old.metadata),
            ),
            condition_functions: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.condition_functions[ranges[4].clone()]),
                previous.map(|old| &old.condition_functions),
            ),
            effect_functions: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.effect_functions[ranges[5].clone()]),
                previous.map(|old| &old.effect_functions),
            ),
            bindings: region_outputs::join_shared(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.bindings[ranges[6].clone()]),
                previous.map(|old| &old.bindings),
            ),
        };
        if let Some(previous) = previous
            && joined.shares_storage(previous)
        {
            Arc::clone(previous)
        } else {
            Arc::new(joined)
        }
    }
    fn shares_storage(&self, previous: &Self) -> bool {
        let Self {
            blocks,
            block_references,
            stable_ids,
            metadata,
            condition_functions,
            effect_functions,
            bindings,
        } = self;
        Arc::ptr_eq(blocks, &previous.blocks)
            && Arc::ptr_eq(block_references, &previous.block_references)
            && Arc::ptr_eq(stable_ids, &previous.stable_ids)
            && Arc::ptr_eq(metadata, &previous.metadata)
            && Arc::ptr_eq(condition_functions, &previous.condition_functions)
            && Arc::ptr_eq(effect_functions, &previous.effect_functions)
            && Arc::ptr_eq(bindings, &previous.bindings)
    }
}

impl AuthoringSummary {
    pub(crate) fn relocate_region(
        &mut self,
        ranges: &SummaryRanges,
        mut shift: impl FnMut(&mut recite_core::SourceSpan),
    ) {
        for item in region_mut(&mut self.blocks, ranges[0].clone()) {
            let item = Arc::make_mut(&mut item.data);
            shift(&mut item.span);
            item.id_span.iter_mut().for_each(&mut shift);
        }
        for item in region_mut(&mut self.block_references, ranges[1].clone()) {
            shift(&mut item.span);
            item.file_span.iter_mut().for_each(&mut shift);
            item.block_id_span.iter_mut().for_each(&mut shift);
        }
        for item in region_mut(&mut self.stable_ids, ranges[2].clone()) {
            shift(&mut item.span);
            item.source_id_span.iter_mut().for_each(&mut shift);
            item.insertion_span.iter_mut().for_each(&mut shift);
        }
        for item in region_mut(&mut self.metadata, ranges[3].clone()) {
            item.source_span.iter_mut().for_each(&mut shift);
            item.key_span.iter_mut().for_each(&mut shift);
            item.value_span.iter_mut().for_each(&mut shift);
            item.value_element_spans.iter_mut().for_each(&mut shift);
        }
        for item in region_mut(&mut self.condition_functions, ranges[4].clone())
            .iter_mut()
            .chain(region_mut(&mut self.effect_functions, ranges[5].clone()))
        {
            shift(&mut item.span);
        }
    }
}

#[cfg(test)]
mod tests;
