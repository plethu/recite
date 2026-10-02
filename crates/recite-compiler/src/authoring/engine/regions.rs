//! Reuse syntax and local analysis at parser-owned restart boundaries.
//! Only ranges into document-owned outputs survive a candidate commit.
use std::{ops::Range, sync::Arc};

use recite_core::{Diagnostic, schema::ProjectSchema};
use recite_parser::{LoweredSourceFile, SourceRegion, source_regions};

use super::{DocumentAnalysis, EffectiveDocument, participation_for};
use crate::{
    authoring::{AuthoringSummary, Interrupted, WorkControl, summary::SummaryRanges},
    validation::{
        ValidationInput, ValidationParticipation,
        incremental::{FactRanges, ProjectFacts, validate_local},
    },
};

mod assembly;
mod matching;
mod relocation;

thread_local! {
    static PARSED_REGIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn lower_region(source: &SourceRegion<'_>, path: &str) -> LoweredSourceFile {
    if cfg!(test) {
        PARSED_REGIONS.with(|count| count.set(count.get() + 1));
    }
    source.parse(path).lower_source_file()
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CachedRegion {
    bytes: Range<usize>,
    first_line: u32,
    participation: ValidationParticipation,
    summary: SummaryRanges,
    facts: FactRanges,
    parse: Range<usize>,
    local: Range<usize>,
}

struct PendingRegion<'a> {
    source: SourceRegion<'a>,
    syntax: RegionSyntax<'a>,
}

enum RegionSyntax<'a> {
    Cached(&'a CachedRegion),
    Lowered(LoweredSourceFile),
}

impl RegionSyntax<'_> {
    fn participation(&self) -> ValidationParticipation {
        match self {
            Self::Cached(cached) => cached.participation,
            Self::Lowered(lowered) => participation_for(lowered.recovery),
        }
    }
}

struct FreshAnalysis {
    summary: AuthoringSummary,
    facts: ProjectFacts,
    parse: Vec<Diagnostic>,
    local: Vec<Diagnostic>,
}

enum RegionAnalysis<'a> {
    Reused(&'a DocumentAnalysis, &'a CachedRegion),
    Fresh(Box<FreshAnalysis>),
}

pub(super) fn analyze(
    document: &EffectiveDocument<'_>,
    previous: Option<&DocumentAnalysis>,
    schema: Option<&ProjectSchema>,
    control: &dyn WorkControl,
) -> Result<DocumentAnalysis, Interrupted> {
    let mut pending = Vec::new();
    let mut participation = ValidationParticipation::all_complete();
    let sources = source_regions(document.text);
    let matches = matching::match_regions(&sources, previous);
    for (source, cached) in sources.into_iter().zip(matches) {
        control.checkpoint()?;
        let syntax = match cached {
            Some(cached) => RegionSyntax::Cached(cached),
            None => RegionSyntax::Lowered(lower_region(&source, document.key.as_str())),
        };
        participation = participation.merge(syntax.participation());
        pending.push(PendingRegion { source, syntax });
    }
    let mut analyses = Vec::with_capacity(pending.len());
    let mut regions = Vec::with_capacity(pending.len());
    for region in pending {
        control.checkpoint()?;
        let region_participation = region.syntax.participation();
        let reused = match (previous, &region.syntax) {
            (Some(old), RegionSyntax::Cached(cached)) if old.participation == participation => {
                Some(RegionAnalysis::Reused(old, cached))
            }
            _ => None,
        };
        let analysis = reused.unwrap_or_else(|| {
            // Recovery participation is file-wide. When it changes, local
            // checks must run again even for byte-identical regions.
            let lowered = match region.syntax {
                RegionSyntax::Lowered(lowered) => lowered,
                RegionSyntax::Cached(_) => lower_region(&region.source, document.key.as_str()),
            };
            let input = ValidationInput::new(&lowered.source_file, participation);
            RegionAnalysis::Fresh(Box::new(FreshAnalysis {
                summary: AuthoringSummary::from_source_file(&lowered.source_file),
                facts: ProjectFacts::collect(input),
                parse: lowered.diagnostics,
                local: validate_local(input, schema).diagnostics,
            }))
        });
        regions.push(CachedRegion {
            bytes: region.source.byte_range(),
            first_line: region.source.first_line(),
            participation: region_participation,
            summary: analysis.summary().1,
            facts: analysis.facts().1,
            parse: 0..analysis.parse().len(),
            local: 0..analysis.local().len(),
        });
        analyses.push(analysis);
    }
    control.checkpoint()?;
    match assembly::assemble(
        document,
        previous,
        participation,
        &analyses,
        regions,
        control,
    )? {
        Some(analysis) => Ok(analysis),
        // An invalid relocated coordinate must never enter a candidate. A
        // cold pass has no relocations and preserves ordinary parser recovery.
        None => analyze(document, None, schema, control),
    }
}

#[cfg(test)]
mod tests;
