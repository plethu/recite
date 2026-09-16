//! Incremental document dependencies, with the batch validator as policy owner.
//! Only changed documents and consumers/colliders of their exports are checked.
use super::{ProjectFacts, facts::Symbol, validate_context};
use crate::validation::project::first_source_span;
use recite_core::{Diagnostic, DocumentKey, SourceFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

type Membership = BTreeMap<Symbol, Vec<Arc<DocumentKey>>>;

#[derive(Debug, Default)]
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
    ) -> BTreeSet<DocumentKey> {
        let changes: Vec<_> = changes.into_iter().collect();
        if changes.is_empty() && self.complete == complete {
            return BTreeSet::new();
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
            self.affect_dependents(key, changed_targets.contains(key), &mut affected);
        }
        for (key, next) in &changes {
            if let Some(old) = self.documents.remove(key) {
                remove(&mut self.definitions, old.exports(), key);
                remove(&mut self.dependents, old.dependencies(), key);
            }
            if let Some(next) = next {
                let member = Arc::new(key.clone());
                insert(&mut self.definitions, next.exports(), &member);
                insert(&mut self.dependents, next.dependencies(), &member);
                self.documents.insert(key.clone(), Arc::clone(next));
            }
        }
        for (key, _) in &changes {
            self.affect_dependents(key, changed_targets.contains(key), &mut affected);
        }
        let stable_complete = self
            .documents
            .values()
            .all(|facts| facts.participation.stable_ids().is_complete());
        if self.complete != complete || self.stable_complete != stable_complete {
            affected.extend(self.documents.keys().cloned());
        }
        self.complete = complete;
        self.stable_complete = stable_complete;
        if let Some(key) = self.first_document() {
            affected.insert(key);
        }
        for key in &affected {
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
        affected
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
fn remove(members: &mut Membership, symbols: impl Iterator<Item = Symbol>, key: &DocumentKey) {
    for symbol in symbols {
        if let Some(entries) = members.get_mut(&symbol) {
            entries.retain(|member| member.as_ref() != key);
            if entries.is_empty() {
                members.remove(&symbol);
            }
        }
    }
}
fn insert(members: &mut Membership, symbols: impl Iterator<Item = Symbol>, key: &Arc<DocumentKey>) {
    for symbol in symbols {
        let entries = members
            .entry(symbol)
            .or_insert_with(|| vec![Arc::clone(key)]);
        if !entries.contains(key) {
            entries.push(Arc::clone(key));
        }
    }
}
