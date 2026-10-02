use recite_core::{Diagnostic, SourcePosition, SourceSpan};

use super::{CachedRegion, DocumentAnalysis, FreshAnalysis, RegionAnalysis};

pub(super) fn reuse(
    old: &DocumentAnalysis,
    cached: &CachedRegion,
    first_line: u32,
    path: &str,
) -> Option<FreshAnalysis> {
    let delta = i64::from(first_line) - i64::from(cached.first_line);
    let mut valid = true;
    let mut shift = |span: &mut SourceSpan| {
        // Local validation can attach schema locations outside this source.
        if span.file != path {
            return;
        }
        match (
            position(span.start, delta),
            span.end.map(|end| position(end, delta)),
        ) {
            (Some(start), None) => span.start = start,
            (Some(start), Some(Some(end))) => {
                span.start = start;
                span.end = Some(end);
            }
            _ => valid = false,
        }
    };
    let summary = old.summary.relocated_region(&cached.summary, &mut shift);
    let facts = old
        .project_facts
        .relocated_region(&cached.facts, &mut shift);
    let region = RegionAnalysis::Reused(old, cached);
    let parse = diagnostics(region.parse(), &mut shift);
    let local = diagnostics(region.local(), &mut shift);
    valid.then_some(FreshAnalysis {
        summary,
        facts,
        parse,
        local,
    })
}

fn position(position: SourcePosition, delta: i64) -> Option<SourcePosition> {
    SourcePosition::new(
        u32::try_from(i64::from(position.line()) + delta).ok()?,
        position.column(),
    )
    .ok()
}

fn diagnostics(source: &[Diagnostic], shift: &mut impl FnMut(&mut SourceSpan)) -> Vec<Diagnostic> {
    let mut result = source.to_vec();
    for diagnostic in &mut result {
        shift(&mut diagnostic.span);
        for related in &mut diagnostic.related {
            shift(&mut related.span);
        }
        for related in &mut diagnostic.related_presentations {
            shift(&mut related.span);
        }
    }
    result
}
