//! Compact offsets into a document summary; cached regions do not duplicate it.
use super::AuthoringSummary;
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
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
                &previous.blocks,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.block_references[ranges[1].clone()]),
                &previous.block_references,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.stable_ids[ranges[2].clone()]),
                &previous.stable_ids,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.metadata[ranges[3].clone()]),
                &previous.metadata,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.condition_functions[ranges[4].clone()]),
                &previous.condition_functions,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.effect_functions[ranges[5].clone()]),
                &previous.effect_functions,
            )
        {
            return Arc::clone(previous);
        }
        Arc::new(Self {
            blocks: parts
                .iter()
                .flat_map(|(part, ranges)| part.blocks[ranges[0].clone()].iter().cloned())
                .collect(),
            block_references: parts
                .iter()
                .flat_map(|(part, ranges)| part.block_references[ranges[1].clone()].iter().cloned())
                .collect(),
            stable_ids: parts
                .iter()
                .flat_map(|(part, ranges)| part.stable_ids[ranges[2].clone()].iter().cloned())
                .collect(),
            metadata: parts
                .iter()
                .flat_map(|(part, ranges)| part.metadata[ranges[3].clone()].iter().cloned())
                .collect(),
            condition_functions: parts
                .iter()
                .flat_map(|(part, ranges)| {
                    part.condition_functions[ranges[4].clone()].iter().cloned()
                })
                .collect(),
            effect_functions: parts
                .iter()
                .flat_map(|(part, ranges)| part.effect_functions[ranges[5].clone()].iter().cloned())
                .collect(),
        })
    }
}

impl AuthoringSummary {
    pub(crate) fn relocated_region(
        &self,
        ranges: &SummaryRanges,
        mut shift: impl FnMut(&mut recite_core::SourceSpan),
    ) -> Self {
        let mut result = Self {
            blocks: self.blocks[ranges[0].clone()].to_vec(),
            block_references: self.block_references[ranges[1].clone()].to_vec(),
            stable_ids: self.stable_ids[ranges[2].clone()].to_vec(),
            metadata: self.metadata[ranges[3].clone()].to_vec(),
            condition_functions: self.condition_functions[ranges[4].clone()].to_vec(),
            effect_functions: self.effect_functions[ranges[5].clone()].to_vec(),
        };
        for item in &mut result.blocks {
            shift(&mut item.span);
            item.id_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut result.block_references {
            shift(&mut item.span);
            item.file_span.iter_mut().for_each(&mut shift);
            item.block_id_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut result.stable_ids {
            shift(&mut item.span);
            item.source_id_span.iter_mut().for_each(&mut shift);
            item.insertion_span.iter_mut().for_each(&mut shift);
        }
        for item in &mut result.metadata {
            item.source_span.iter_mut().for_each(&mut shift);
            item.key_span.iter_mut().for_each(&mut shift);
            item.value_span.iter_mut().for_each(&mut shift);
            item.value_element_spans.iter_mut().for_each(&mut shift);
        }
        for item in result
            .condition_functions
            .iter_mut()
            .chain(&mut result.effect_functions)
        {
            shift(&mut item.span);
        }
        result
    }
}
