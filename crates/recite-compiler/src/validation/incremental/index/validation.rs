//! Share validation contexts without materializing an entire large project's AST.
use super::{
    Arc, BTreeSet, DocumentKey, Interrupted, ProjectDiagnostics, ProjectIndex, WorkControl,
    extend_members,
};
use crate::validation::incremental::validate_context;

// Compact facts expand into temporary AST statements during validation. Bound
// shared batches by passage count, not file count: one file may contain a novel.
// An individual target's required context is never truncated to fit the budget.
const MAX_CONTEXT_PASSAGES: usize = 3_072;

impl ProjectIndex {
    pub(super) fn revalidate(
        &self,
        affected: &BTreeSet<DocumentKey>,
        diagnostics: &mut ProjectDiagnostics,
        control: &dyn WorkControl,
    ) -> Result<(), Interrupted> {
        let mut context = BTreeSet::new();
        let mut targets = BTreeSet::new();
        let mut passages = 0;
        for key in affected {
            control.checkpoint()?;
            diagnostics.remove(key);
            let Some(target) = self.documents.get(key) else {
                continue;
            };
            let mut next = BTreeSet::from([key.clone()]);
            // Memberships contain only current keys. Once every document is
            // present, more symbol lookups cannot enlarge this context.
            if next.len() < self.documents.len() {
                for symbol in target.exports().chain(target.dependencies()) {
                    extend_members(&self.definitions, &symbol, &mut next);
                    if next.len() == self.documents.len() {
                        break;
                    }
                }
            }
            let additional = self.passage_count(next.difference(&context));
            if !targets.is_empty() && additional > 0 && passages + additional > MAX_CONTEXT_PASSAGES
            {
                self.validate_batch(&context, &targets, diagnostics, control)?;
                context.clear();
                targets.clear();
                passages = 0;
            }
            passages += self.passage_count(next.difference(&context));
            context.extend(next);
            targets.insert(key.clone());
        }
        self.validate_batch(&context, &targets, diagnostics, control)
    }

    fn passage_count<'a>(&self, keys: impl Iterator<Item = &'a DocumentKey>) -> usize {
        keys.filter_map(|key| self.documents.get(key))
            .map(|facts| facts.passages.len())
            .sum()
    }

    fn validate_batch(
        &self,
        context: &BTreeSet<DocumentKey>,
        targets: &BTreeSet<DocumentKey>,
        diagnostics: &mut ProjectDiagnostics,
        control: &dyn WorkControl,
    ) -> Result<(), Interrupted> {
        if targets.is_empty() {
            return Ok(());
        }
        control.checkpoint()?;
        let facts: Vec<_> = context
            .iter()
            .filter_map(|key| self.documents.get(key).map(Arc::as_ref))
            .collect();
        if cfg!(test) {
            MAX_VALIDATED_PASSAGES.with(|count| {
                count.set(count.get().max(self.passage_count(context.iter())));
            });
        }
        // Context providers may lack their own dependencies. Publish only the
        // batch's target diagnostics, whose complete contexts are present.
        let reports = validate_context(&facts, self.complete, self.stable_complete);
        for (references, reports) in [(false, reports.structural), (true, reports.references)] {
            for diagnostic in Arc::unwrap_or_clone(reports) {
                control.checkpoint()?;
                if let Some(key) = targets.get(diagnostic.span.file.as_str()) {
                    let reports = diagnostics.entry(key.clone()).or_default();
                    if references {
                        Arc::make_mut(&mut reports.references).push(diagnostic);
                    } else {
                        Arc::make_mut(&mut reports.structural).push(diagnostic);
                    }
                }
            }
        }
        Ok(())
    }
}

thread_local! {
    static MAX_VALIDATED_PASSAGES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests;
