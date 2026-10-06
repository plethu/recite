//! Composition of region facts without retaining duplicate per-region arrays.
use super::ProjectFacts;
use crate::region_outputs;
use crate::validation::ValidationParticipation;
use std::{ops::Range, sync::Arc};

pub(crate) type FactRanges = [Range<usize>; 3];
impl ProjectFacts {
    pub(crate) fn region_ranges(&self) -> FactRanges {
        [
            0..self.blocks.len(),
            0..self.passages.len(),
            0..self.references.len(),
        ]
    }
    pub(crate) fn join_regions(
        path: &str,
        participation: ValidationParticipation,
        parts: &[(&Self, &FactRanges)],
        previous: Option<&Arc<Self>>,
    ) -> Arc<Self> {
        if let Some(previous) = previous
            && previous.participation == participation
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
                &previous.blocks,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.passages[ranges[1].clone()]),
                &previous.passages,
            )
            && region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.references[ranges[2].clone()]),
                &previous.references,
            )
        {
            return Arc::clone(previous);
        }
        Arc::new(Self {
            path: path.into(),
            participation,
            blocks: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
            )
            .into_boxed_slice(),
            passages: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.passages[ranges[1].clone()]),
            )
            .into_boxed_slice(),
            references: region_outputs::collect(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.references[ranges[2].clone()]),
            )
            .into_boxed_slice(),
        })
    }
}

impl ProjectFacts {
    pub(crate) fn relocate_region(
        &mut self,
        ranges: &FactRanges,
        mut shift: impl FnMut(&mut recite_core::SourceSpan),
    ) {
        for item in &mut self.blocks[ranges[0].clone()] {
            shift(&mut item.span);
        }
        for item in &mut self.passages[ranges[1].clone()] {
            shift(&mut item.span);
        }
        for item in &mut self.references[ranges[2].clone()] {
            shift(&mut item.span);
            if let recite_core::ast::DivertTarget::Block(target) = &mut item.target {
                target.file_span.iter_mut().for_each(&mut shift);
                target.block_id_span.iter_mut().for_each(&mut shift);
            }
        }
    }
}
