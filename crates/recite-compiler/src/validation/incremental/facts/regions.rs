//! Composition of region facts without retaining duplicate per-region arrays.
use super::ProjectFacts;
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
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.blocks[ranges[0].clone()]),
                &previous.blocks,
            )
            && crate::region_outputs::matches(
                parts
                    .iter()
                    .map(|(part, ranges)| &part.passages[ranges[1].clone()]),
                &previous.passages,
            )
            && crate::region_outputs::matches(
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
            blocks: parts
                .iter()
                .flat_map(|(part, ranges)| part.blocks[ranges[0].clone()].iter().cloned())
                .collect(),
            passages: parts
                .iter()
                .flat_map(|(part, ranges)| part.passages[ranges[1].clone()].iter().cloned())
                .collect(),
            references: parts
                .iter()
                .flat_map(|(part, ranges)| part.references[ranges[2].clone()].iter().cloned())
                .collect(),
        })
    }
}

impl ProjectFacts {
    pub(crate) fn relocated_region(
        &self,
        ranges: &FactRanges,
        mut shift: impl FnMut(&mut recite_core::SourceSpan),
    ) -> Self {
        let mut result = Self {
            path: self.path.clone(),
            participation: self.participation,
            blocks: self.blocks[ranges[0].clone()].into(),
            passages: self.passages[ranges[1].clone()].into(),
            references: self.references[ranges[2].clone()].into(),
        };
        for item in &mut result.blocks {
            shift(&mut item.span);
        }
        for item in &mut result.passages {
            shift(&mut item.span);
        }
        for item in &mut result.references {
            shift(&mut item.span);
            if let recite_core::ast::DivertTarget::Block(target) = &mut item.target {
                target.file_span.iter_mut().for_each(&mut shift);
                target.block_id_span.iter_mut().for_each(&mut shift);
            }
        }
        result
    }
}
