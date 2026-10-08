use recite_core::{Diagnostic, SourcePosition, SourceSpan};

use super::{AnalyzedRegion, DocumentAnalysis, Interrupted, RegionAnalysis, WorkControl};
use std::sync::Arc;

pub(super) fn apply(
    analysis: &mut DocumentAnalysis,
    regions: &[AnalyzedRegion<'_>],
    path: &str,
    control: &dyn WorkControl,
) -> Result<bool, Interrupted> {
    let mut valid = true;
    for region in regions {
        control.checkpoint()?;
        let RegionAnalysis::Reused(_, cached) = &region.output else {
            continue;
        };
        let delta = i64::from(region.cache.first_line) - i64::from(cached.first_line);
        if delta == 0 {
            continue;
        }
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
        Arc::make_mut(&mut analysis.summary).relocate_region(&region.cache.summary, &mut shift);
        Arc::make_mut(&mut analysis.project_facts).relocate_region(&region.cache.facts, &mut shift);
        diagnostics(
            &mut Arc::make_mut(&mut analysis.parse_diagnostics)[region.cache.parse.clone()],
            &mut shift,
        );
        diagnostics(
            &mut Arc::make_mut(&mut analysis.local_diagnostics)[region.cache.local.clone()],
            &mut shift,
        );
    }
    Ok(valid)
}

fn position(position: SourcePosition, delta: i64) -> Option<SourcePosition> {
    SourcePosition::new(
        u32::try_from(i64::from(position.line()) + delta).ok()?,
        position.column(),
    )
    .ok()
}

fn diagnostics(source: &mut [Diagnostic], shift: &mut impl FnMut(&mut SourceSpan)) {
    for diagnostic in source {
        shift(&mut diagnostic.span);
        for related in &mut diagnostic.related {
            shift(&mut related.span);
        }
        for related in &mut diagnostic.related_presentations {
            shift(&mut related.span);
        }
    }
}
