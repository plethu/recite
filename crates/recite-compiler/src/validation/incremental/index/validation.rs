//! Share validation contexts without materializing an entire large project's AST.
use super::{
    BTreeMap, BTreeSet, Diagnostic, DocumentKey, Interrupted, ProjectIndex, WorkControl,
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
        diagnostics: &mut BTreeMap<DocumentKey, Vec<Diagnostic>>,
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
            for symbol in target.exports().chain(target.dependencies()) {
                extend_members(&self.definitions, &symbol, &mut next);
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
        diagnostics: &mut BTreeMap<DocumentKey, Vec<Diagnostic>>,
        control: &dyn WorkControl,
    ) -> Result<(), Interrupted> {
        if targets.is_empty() {
            return Ok(());
        }
        control.checkpoint()?;
        let facts: Vec<_> = context
            .iter()
            .filter_map(|key| self.documents.get(key).map(std::sync::Arc::as_ref))
            .collect();
        if cfg!(test) {
            MAX_VALIDATED_PASSAGES.with(|count| {
                count.set(count.get().max(self.passage_count(context.iter())));
            });
        }
        // Context providers may lack their own dependencies. Publish only the
        // batch's target diagnostics, whose complete contexts are present.
        for diagnostic in validate_context(&facts, self.complete, self.stable_complete) {
            control.checkpoint()?;
            if let Some(key) = targets.get(diagnostic.span.file.as_str()) {
                diagnostics.entry(key.clone()).or_default().push(diagnostic);
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
