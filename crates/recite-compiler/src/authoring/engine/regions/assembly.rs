use super::*;

impl RegionAnalysis<'_> {
    pub(super) fn summary(&self) -> (&AuthoringSummary, SummaryRanges) {
        match self {
            Self::Reused(old, region) => (&old.summary, region.summary.clone()),
            Self::Fresh(fresh) => (&fresh.summary, fresh.summary.region_ranges()),
        }
    }
    pub(super) fn facts(&self) -> (&ProjectFacts, FactRanges) {
        match self {
            Self::Reused(old, region) => (&old.project_facts, region.facts.clone()),
            Self::Fresh(fresh) => (&fresh.facts, fresh.facts.region_ranges()),
        }
    }
    pub(super) fn parse(&self) -> &[Diagnostic] {
        match self {
            Self::Reused(old, region) => &old.parse_diagnostics[region.parse.clone()],
            Self::Fresh(fresh) => &fresh.parse,
        }
    }
    pub(super) fn local(&self) -> &[Diagnostic] {
        match self {
            Self::Reused(old, region) => &old.local_diagnostics[region.local.clone()],
            Self::Fresh(fresh) => &fresh.local,
        }
    }
}

pub(super) fn assemble(
    document: &EffectiveDocument<'_>,
    previous: Option<&DocumentAnalysis>,
    participation: ValidationParticipation,
    analyses: &[RegionAnalysis<'_>],
    mut regions: Vec<CachedRegion>,
    control: &dyn WorkControl,
) -> Result<Option<DocumentAnalysis>, Interrupted> {
    let relocated = analyses.iter().zip(&regions).any(|(part, region)| {
        matches!(part, RegionAnalysis::Reused(_, cached) if cached.first_line != region.first_line)
    });
    // Shifted output needs a fresh allocation even if the unshifted slices
    // compare equal. Never mutate an earlier immutable snapshot.
    let previous = previous.filter(|_| !relocated);
    let summaries: Vec<_> = analyses.iter().map(RegionAnalysis::summary).collect();
    let facts: Vec<_> = analyses.iter().map(RegionAnalysis::facts).collect();
    let summary = AuthoringSummary::join_regions(
        &summaries
            .iter()
            .map(|(part, range)| (*part, range))
            .collect::<Vec<_>>(),
        previous.map(|old| &old.summary),
    );
    let project_facts = ProjectFacts::join_regions(
        document.key.as_str(),
        participation,
        &facts
            .iter()
            .map(|(part, range)| (*part, range))
            .collect::<Vec<_>>(),
        previous.map(|old| &old.project_facts),
    );
    let mut summary_offset = [0; 6];
    let mut facts_offset = [0; 3];
    let mut parse_offset = 0;
    let mut local_offset = 0;
    for region in &mut regions {
        for (range, offset) in region.summary.iter_mut().zip(&mut summary_offset) {
            advance(range, offset);
        }
        for (range, offset) in region.facts.iter_mut().zip(&mut facts_offset) {
            advance(range, offset);
        }
        advance(&mut region.parse, &mut parse_offset);
        advance(&mut region.local, &mut local_offset);
    }
    let mut analysis = DocumentAnalysis {
        regions: regions.into(),
        summary,
        project_facts,
        participation,
        source: Arc::new(recite_core::SourceLineIndex::new(Arc::clone(document.text))),
        source_fingerprint: crate::authoring::SourceFingerprint::for_source(document.text),
        parse_diagnostics: analyses
            .iter()
            .flat_map(|part| part.parse().iter().cloned())
            .collect(),
        local_diagnostics: analyses
            .iter()
            .flat_map(|part| part.local().iter().cloned())
            .collect(),
        byte_len: document.text.len(),
        line_count: recite_core::source_lines(document.text)
            .filter(|(content, terminator)| !content.is_empty() || !terminator.is_empty())
            .count(),
    };
    if relocated && !relocation::apply(&mut analysis, analyses, document.key.as_str(), control)? {
        return Ok(None);
    }
    Ok(Some(analysis))
}

fn advance(range: &mut Range<usize>, offset: &mut usize) {
    let end = *offset + range.len();
    *range = *offset..end;
    *offset = end;
}
