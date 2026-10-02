//! Incremental document dependencies, with the batch validator as policy owner.
//! Only changed documents and consumers/colliders of their exports are checked.
use super::{ProjectFacts, facts::Symbol, validate_context};
use crate::authoring::{Interrupted, WorkControl};
use crate::validation::project::first_source_span;
use recite_core::{Diagnostic, DocumentKey, ast::SourceFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

mod relocation;

// Candidates share symbol keys and memberships; only affected entries copy.
type Membership = rpds::RedBlackTreeMapSync<Symbol, Vec<Arc<DocumentKey>>>;

#[derive(Clone, Debug, Default)]
pub(crate) struct ProjectIndex {
    documents: BTreeMap<DocumentKey, Arc<ProjectFacts>>,
    definitions: Membership,
    dependents: Membership,
    complete: bool,
    stable_complete: bool,
}
impl ProjectIndex {
    pub(crate) fn update(
        &mut self,
        changes: impl IntoIterator<Item = (DocumentKey, Option<Arc<ProjectFacts>>)>,
        complete: bool,
        diagnostics: &mut BTreeMap<DocumentKey, Vec<Diagnostic>>,
        control: &dyn WorkControl,
    ) -> Result<BTreeSet<DocumentKey>, Interrupted> {
        let changes: Vec<_> = changes.into_iter().collect();
        if changes.is_empty() && self.complete == complete {
            return Ok(BTreeSet::new());
        }
        if self.complete == complete
            && let Some(affected) = self.relocate_only(&changes, diagnostics, control)?
        {
            return Ok(affected);
        }
        let changed_targets: BTreeSet<_> = changes
            .iter()
            .filter(|(key, next)| match (self.documents.get(key), next) {
                (Some(old), Some(next)) => !old.same_block_targets(next),
                _ => true,
            })
            .map(|(key, _)| key.clone())
            .collect();
        let mut affected: BTreeSet<_> = changes.iter().map(|(key, _)| key.clone()).collect();
        if let Some(key) = self.first_document() {
            affected.insert(key);
        }
        // Capture old dependencies before removing exports, including deleted
        // targets and diagnostics whose related span came from the old revision.
        for (key, _) in &changes {
            control.checkpoint()?;
            self.affect_dependents(key, changed_targets.contains(key), &mut affected);
        }
        for (key, next) in &changes {
            control.checkpoint()?;
            let old = self.documents.remove(key);
            replace_membership(
                &mut self.definitions,
                old.iter().flat_map(|facts| facts.exports()),
                next.iter().flat_map(|facts| facts.exports()),
                key,
            );
            replace_membership(
                &mut self.dependents,
                old.iter().flat_map(|facts| facts.dependencies()),
                next.iter().flat_map(|facts| facts.dependencies()),
                key,
            );
            if let Some(next) = next {
                self.documents.insert(key.clone(), Arc::clone(next));
            }
        }
        for (key, _) in &changes {
            control.checkpoint()?;
            self.affect_dependents(key, changed_targets.contains(key), &mut affected);
        }
        let stable_complete = self
            .documents
            .values()
            .all(|facts| facts.participation.stable_ids().is_complete());
        if self.complete != complete {
            affected.extend(self.documents.keys().cloned());
        } else if self.stable_complete != stable_complete {
            // The project-wide stable-ID gate belongs to choice-echo lookup.
            // Other documents cannot change diagnostics solely from this gate.
            affected.extend(
                self.documents
                    .iter()
                    .filter(|(_, facts)| facts.depends_on_stable_completeness())
                    .map(|(key, _)| key.clone()),
            );
        }
        self.complete = complete;
        self.stable_complete = stable_complete;
        if let Some(key) = self.first_document() {
            affected.insert(key);
        }
        for key in &affected {
            control.checkpoint()?;
            diagnostics.remove(key);
            let Some(target) = self.documents.get(key) else {
                continue;
            };
            let mut context = BTreeSet::from([key.clone()]);
            for symbol in target.exports().chain(target.dependencies()) {
                extend_members(&self.definitions, &symbol, &mut context);
            }
            let facts: Vec<_> = context
                .iter()
                .filter_map(|key| self.documents.get(key).map(Arc::as_ref))
                .collect();
            // Context providers need not have all of their own dependencies in
            // this projection. Only the target's diagnostics may be published.
            let report: Vec<_> = validate_context(&facts, complete, stable_complete)
                .into_iter()
                .filter(|diagnostic| diagnostic.span.file == key.as_str())
                .collect();
            if !report.is_empty() {
                diagnostics.insert(key.clone(), report);
            }
        }
        if let Some((key, diagnostic)) = self.missing_default() {
            diagnostics.entry(key).or_default().push(diagnostic);
        }
        Ok(affected)
    }

    fn affect_dependents(
        &self,
        key: &DocumentKey,
        targets_changed: bool,
        affected: &mut BTreeSet<DocumentKey>,
    ) {
        if let Some(facts) = self.documents.get(key) {
            for symbol in facts.exports() {
                extend_members(&self.definitions, &symbol, affected);
                if targets_changed || !matches!(symbol, Symbol::Document(_)) {
                    extend_members(&self.dependents, &symbol, affected);
                }
            }
        }
    }

    fn first_document(&self) -> Option<DocumentKey> {
        self.documents
            .iter()
            .find(|(_, facts)| !facts.blocks.is_empty())
            .or_else(|| self.documents.first_key_value())
            .map(|(key, _)| key.clone())
    }

    fn missing_default(&self) -> Option<(DocumentKey, Diagnostic)> {
        if !self.complete
            || self.documents.values().any(|facts| {
                !facts.participation.block_definitions().is_complete()
                    || facts.blocks.iter().any(|block| block.default)
            })
        {
            return None;
        }
        let key = self.first_document()?;
        let facts = self.documents.get(&key)?;
        let span = facts
            .blocks
            .first()
            .map(|block| block.span.clone())
            .unwrap_or_else(|| first_source_span([&SourceFile::new(&facts.path, Vec::new())]));
        Some((key, crate::diagnostics::missing_default_block(span)))
    }
}

fn extend_members(members: &Membership, symbol: &Symbol, documents: &mut BTreeSet<DocumentKey>) {
    if let Some(members) = members.get(symbol) {
        documents.extend(members.iter().map(|key| key.as_ref().clone()));
    }
}
fn replace_membership(
    members: &mut Membership,
    old: impl Iterator<Item = Symbol>,
    next: impl Iterator<Item = Symbol>,
    key: &DocumentKey,
) {
    let old: BTreeSet<_> = old.collect();
    let next: BTreeSet<_> = next.collect();
    if old == next {
        return;
    }
    for symbol in old.difference(&next) {
        if let Some(entries) = members.get_mut(symbol) {
            entries.retain(|member| member.as_ref() != key);
            if entries.is_empty() {
                members.remove_mut(symbol);
            }
        }
    }
    let key = Arc::new(key.clone());
    for symbol in next.difference(&old) {
        if let Some(entries) = members.get_mut(symbol) {
            entries.push(Arc::clone(&key));
        } else {
            members.insert_mut(symbol.clone(), vec![Arc::clone(&key)]);
        }
    }
}
