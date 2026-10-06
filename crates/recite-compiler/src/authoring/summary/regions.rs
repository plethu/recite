//! Compact offsets into a document summary; cached regions do not duplicate it.
use super::AuthoringSummary;
use crate::region_outputs;
use std::{ops::Range, sync::Arc};

pub(crate) type SummaryRanges = [Range<usize>; 6];

impl AuthoringSummary {
    pub(crate) fn region_ranges(&self) -> SummaryRanges {
        [
            0..self.blocks.len(),
            0..self.block_references.len(),
            0..self.stable_ids.len(),
            0..self.metadata.len(),
            0..self.condition_functions.len(),
            0..self.effect_functions.len(),
        ]
    }

    pub(crate) fn join_regions(
        parts: &[(&Self, &SummaryRanges)],
        previous: Option<&Arc<Self>>,
    ) -> Arc<Self> {
        if let Some(previous) = previous
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
                &previous.blocks,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.block_references[ranges[1].clone()]),
                &previous.block_references,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.stable_ids[ranges[2].clone()]),
                &previous.stable_ids,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.metadata[ranges[3].clone()]),
                &previous.metadata,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.condition_functions[ranges[4].clone()]),
                &previous.condition_functions,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.effect_functions[ranges[5].clone()]),
                &previous.effect_functions,
            )
        {
            return Arc::clone(previous);
        }
        Arc::new(Self {
            blocks: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
            ),
            block_references: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.block_references[ranges[1].clone()]),
            ),
            stable_ids: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.stable_ids[ranges[2].clone()]),
            ),
            metadata: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.metadata[ranges[3].clone()]),
            ),
            condition_functions: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.condition_functions[ranges[4].clone()]),
            ),
            effect_functions: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.effect_functions[ranges[5].clone()]),
            ),
        })
    }
}

impl AuthoringSummary {
    pub(crate) fn relocate_region(
        &mut self,
        ranges: &SummaryRanges,
        mut shift: impl FnMut(&mut recite_core::SourceSpan),
    ) {
        for item in &mut self.blocks[ranges[0].clone()] {
            shift(&mut item.span);
            item.id_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut self.block_references[ranges[1].clone()] {
            shift(&mut item.span);
            item.file_span.iter_mut().for_each(&mut shift);
            item.block_id_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut self.stable_ids[ranges[2].clone()] {
            shift(&mut item.span);
            item.source_id_span.iter_mut().for_each(&mut shift);
            item.insertion_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut self.metadata[ranges[3].clone()] {
            item.source_span.iter_mut().for_each(&mut shift);
            item.key_span.iter_mut().for_each(&mut shift);
            item.value_span.iter_mut().for_each(&mut shift);
            item.value_element_spans.iter_mut().for_each(&mut shift);
        }
        for item in self.condition_functions[ranges[4].clone()]
            .iter_mut()
            .chain(&mut self.effect_functions[ranges[5].clone()])
        {
            shift(&mut item.span);
        }
    }
}
