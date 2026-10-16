//! Reference edits cannot change declaration diagnostics when every other fact is exact.
use super::{
    Arc, BTreeSet, DocumentKey, Interrupted, ProjectDiagnostics, ProjectFacts, ProjectIndex,
    WorkControl, replace_membership,
};
use crate::validation::incremental::{borrowed, facts::Symbol};

impl ProjectIndex {
    pub(super) fn references_only(
        &mut self,
        changes: &[(DocumentKey, Option<Arc<ProjectFacts>>)],
        diagnostics: &mut ProjectDiagnostics,
        control: &dyn WorkControl,
    ) -> Result<Option<BTreeSet<DocumentKey>>, Interrupted> {
        for (key, next) in changes {
            control.checkpoint()?;
            let Some((old, next)) = self.documents.get(key).zip(next.as_ref()) else {
                return Ok(None);
            };
            if !old.same_non_reference_facts(next) {
                return Ok(None);
            }
        }
        let targets: BTreeSet<_> = changes.iter().map(|(key, _)| key.clone()).collect();
        for (key, next) in changes {
            control.checkpoint()?;
            if let Some(next) = next {
                if let Some(old) = self.documents.get(key) {
                    replace_membership(
                        &mut self.dependents,
                        old.reference_dependencies(),
                        next.reference_dependencies(),
                        key,
                    );
                }
                self.documents.insert(key.clone(), Arc::clone(next));
            }
        }
        // Only reference providers are needed. Walking exports would revisit all
        // unchanged passage IDs, and echoes cannot affect these checks.
        let mut context = targets.clone();
        for key in &targets {
            if let Some(facts) = self.documents.get(key) {
                for symbol in facts.reference_dependencies() {
                    control.checkpoint()?;
                    if let Symbol::Document(path) = symbol
                        && let Some((key, _)) = self.documents.get_key_value(path.as_str())
                    {
                        context.insert(key.clone());
                    }
                }
            }
        }
        let facts: Vec<_> = context
            .iter()
            .filter_map(|key| self.documents.get(key).map(Arc::as_ref))
            .collect();
        let references = borrowed::validate_references(&facts, &targets, self.complete);
        control.checkpoint()?;
        for key in &targets {
            if let Some(reports) = diagnostics.get_mut(key) {
                reports.references = Arc::default();
            }
        }
        for diagnostic in references {
            control.checkpoint()?;
            if let Some(key) = targets.get(diagnostic.span.file.as_str()) {
                Arc::make_mut(&mut diagnostics.entry(key.clone()).or_default().references)
                    .push(diagnostic);
            }
        }
        Ok(Some(targets))
    }
}

#[cfg(test)]
mod tests;
