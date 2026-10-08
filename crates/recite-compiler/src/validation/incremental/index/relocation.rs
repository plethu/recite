//! Preserve project diagnostics when only their source positions changed.
use super::{
    Arc, BTreeMap, BTreeSet, Diagnostic, DocumentKey, Interrupted, ProjectFacts, ProjectIndex,
    WorkControl,
};
use recite_core::{SourcePosition, SourceSpan};

type SpanMap<'a> = BTreeMap<(&'a str, SourcePosition, Option<SourcePosition>), &'a SourceSpan>;

fn span_key(span: &SourceSpan) -> (&str, SourcePosition, Option<SourcePosition>) {
    (&span.file, span.start, span.end)
}

impl ProjectIndex {
    pub(super) fn relocate_only(
        &mut self,
        changes: &[(DocumentKey, Option<Arc<ProjectFacts>>)],
        diagnostics: &mut BTreeMap<DocumentKey, Vec<Diagnostic>>,
        control: &dyn WorkControl,
    ) -> Result<Option<BTreeSet<DocumentKey>>, Interrupted> {
        for (key, next) in changes {
            control.checkpoint()?;
            if !self
                .documents
                .get(key)
                .zip(next.as_ref())
                .is_some_and(|(old, next)| old.same_semantics(next))
            {
                return Ok(None);
            }
        }
        let mut maps = BTreeMap::new();
        for (key, next) in changes {
            control.checkpoint()?;
            if !diagnostics
                .values()
                .flatten()
                .any(|diagnostic| spans(diagnostic).any(|span| span.file == key.as_str()))
            {
                continue;
            }
            let Some((old, next)) = self.documents.get(key).zip(next.as_ref()) else {
                return Ok(None);
            };
            let mut map = SpanMap::new();
            for (before, after) in old.diagnostic_spans().zip(next.diagnostic_spans()) {
                if let Some(existing) = map.insert(span_key(before), after)
                    && existing != after
                {
                    return Ok(None);
                }
            }
            maps.insert(key.as_str(), map);
        }
        let mut affected: BTreeSet<_> = changes.iter().map(|(key, _)| key.clone()).collect();
        let mut relocated = diagnostics.clone();
        for (key, reports) in &mut relocated {
            for diagnostic in reports.iter_mut() {
                control.checkpoint()?;
                let mut valid = true;
                for span in std::iter::once(&mut diagnostic.span)
                    .chain(diagnostic.related.iter_mut().map(|item| &mut item.span))
                    .chain(
                        diagnostic
                            .related_presentations
                            .iter_mut()
                            .map(|item| &mut item.span),
                    )
                {
                    if let Some(map) = maps.get(span.file.as_str()) {
                        if let Some(next) = map.get(&span_key(span)) {
                            *span = (*next).clone();
                            affected.insert(key.clone());
                        } else {
                            valid = false;
                        }
                    }
                }
                if !valid {
                    return Ok(None);
                }
            }
            // Position changes may change diagnostic ordering in a document.
            super::super::super::project::sort_diagnostics_by_source(reports);
        }
        // All validation and mapping succeeded before changing the candidate.
        for (key, next) in changes {
            if let Some(next) = next {
                self.documents.insert(key.clone(), Arc::clone(next));
            }
        }
        *diagnostics = relocated;
        Ok(Some(affected))
    }
}

fn spans(diagnostic: &Diagnostic) -> impl Iterator<Item = &SourceSpan> {
    std::iter::once(&diagnostic.span)
        .chain(diagnostic.related.iter().map(|item| &item.span))
        .chain(
            diagnostic
                .related_presentations
                .iter()
                .map(|item| &item.span),
        )
}
