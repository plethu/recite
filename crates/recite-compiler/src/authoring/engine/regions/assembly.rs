use std::{ops::Range, sync::Arc};

use recite_core::{SourceLineIndex, source_lines};

use super::{
    AnalyzedRegion, AnalyzedRegions, AuthoringSummary, DocumentAnalysis, EffectiveDocument,
    Interrupted, ProjectFacts, RegionAnalysis, WorkControl, relocation,
};
use crate::authoring::SourceFingerprint;

pub(super) enum AssemblyOutcome {
    Complete(DocumentAnalysis),
    /// Cached coordinates cannot safely be relocated; parse the whole file.
    NeedsFullAnalysis,
}

pub(super) fn assemble(
    document: &EffectiveDocument<'_>,
    previous: Option<&DocumentAnalysis>,
    analyzed: AnalyzedRegions<'_>,
    control: &dyn WorkControl,
) -> Result<AssemblyOutcome, Interrupted> {
    let AnalyzedRegions {
        participation,
        mut regions,
    } = analyzed;
    let relocated = regions.iter().any(|region| {
        matches!(&region.output, RegionAnalysis::Reused(_, cached) if cached.first_line != region.cache.first_line)
    });
    // Shifted output needs a fresh allocation even when unshifted slices match.
    // Never mutate an earlier immutable snapshot.
    let previous = previous.filter(|_| !relocated);
    let summaries: Vec<_> = regions
        .iter()
        .map(|region| (region.output.summary(), &region.cache.summary))
        .collect();
    let facts: Vec<_> = regions
        .iter()
        .map(|region| (region.output.facts(), &region.cache.facts))
        .collect();
    let summary = AuthoringSummary::join_regions(&summaries, previous.map(|old| &old.summary));
    let project_facts = ProjectFacts::join_regions(
        document.key.as_str(),
        participation,
        &facts,
        previous.map(|old| &old.project_facts),
    );
    rebase_ranges(&mut regions);
    let mut analysis = DocumentAnalysis {
        regions: regions.iter().map(|region| region.cache.clone()).collect(),
        summary,
        project_facts,
        participation,
        source: Arc::new(SourceLineIndex::new(Arc::clone(document.text))),
        source_fingerprint: SourceFingerprint::for_source(document.text),
        parse_diagnostics: regions
            .iter()
            .flat_map(|region| region.output.parse().iter().cloned())
            .collect(),
        local_diagnostics: regions
            .iter()
            .flat_map(|region| region.output.local().iter().cloned())
            .collect(),
        byte_len: document.text.len(),
        line_count: source_lines(document.text)
            .filter(|(content, terminator)| !content.is_empty() || !terminator.is_empty())
            .count(),
    };
    if relocated && !relocation::apply(&mut analysis, &regions, document.key.as_str(), control)? {
        return Ok(AssemblyOutcome::NeedsFullAnalysis);
    }
    Ok(AssemblyOutcome::Complete(analysis))
}

/// Convert each region's local slice lengths into offsets in joined outputs.
fn rebase_ranges(regions: &mut [AnalyzedRegion<'_>]) {
    let mut summary_offsets = std::array::from_fn(|_| 0);
    let mut fact_offsets = std::array::from_fn(|_| 0);
    let mut parse_offset = 0;
    let mut local_offset = 0;
    for region in regions {
        rebase_group(&mut region.cache.summary, &mut summary_offsets);
        rebase_group(&mut region.cache.facts, &mut fact_offsets);
        advance(&mut region.cache.parse, &mut parse_offset);
        advance(&mut region.cache.local, &mut local_offset);
    }
}

fn rebase_group<const N: usize>(ranges: &mut [Range<usize>; N], offsets: &mut [usize; N]) {
    for (range, offset) in ranges.iter_mut().zip(offsets) {
        advance(range, offset);
    }
}

fn advance(range: &mut Range<usize>, offset: &mut usize) {
    let end = *offset + range.len();
    *range = *offset..end;
    *offset = end;
}
