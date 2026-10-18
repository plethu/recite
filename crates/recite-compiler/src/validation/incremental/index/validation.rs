//! Share borrowed validation contexts while bounding their temporary lookup indexes.
use super::{
    Arc, BTreeSet, DocumentKey, Interrupted, ProjectDiagnostics, ProjectIndex, WorkControl,
    extend_members,
};
use crate::validation::incremental::validate_context;

// Bound ordinary batches by passage count, not file count. When one required
// provider already exceeds the bound, permit one bounded batch of additional
// passages so each tiny dependent does not revalidate that same provider.
// Required contexts are never truncated; oversized providers still bound memory.
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
        let mut limit = MAX_CONTEXT_PASSAGES;
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
            if !targets.is_empty() && additional > 0 && passages + additional > limit {
                self.validate_batch(&context, &targets, diagnostics, control)?;
                context.clear();
                targets.clear();
                passages = 0;
            }
            if targets.is_empty() {
                let required = self.passage_count(next.iter());
                limit = if required > MAX_CONTEXT_PASSAGES {
                    required.saturating_add(MAX_CONTEXT_PASSAGES)
                } else {
                    MAX_CONTEXT_PASSAGES
                };
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
            VALIDATED_BATCHES.with(|count| count.set(count.get() + 1));
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
    static VALIDATED_BATCHES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests;
