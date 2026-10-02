use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recite_core::{Diagnostic, SourceRecovery, schema::ProjectSchema};
mod regions;
pub(crate) use regions::CachedRegion;

use super::input_state::EffectiveDocument;
use super::snapshot::{AuthoringSnapshot, DocumentDelta, DocumentSnapshot, delta, metadata};
use super::state::DocumentAnalysis;
use super::{Interrupted, WorkControl};
use crate::validation::ValidationParticipation;
use crate::validation::project::sort_diagnostics_by_source;

use std::cell::Cell;

thread_local! {
    pub(super) static ANALYZE_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static PROJECT_VALIDATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn rebuild_analyses(
    old: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    old_effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    new_effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    schema: Option<&ProjectSchema>,
    control: &dyn WorkControl,
) -> Result<
    (
        BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
        BTreeSet<recite_core::DocumentKey>,
    ),
    Interrupted,
> {
    let mut project_changed = BTreeSet::new();
    let mut analyses = BTreeMap::new();
    for (key, document) in new_effective {
        control.checkpoint()?;
        let previous = old.get(*key);
        let analysis = match previous {
            Some(previous)
                if old_effective.get(key).is_some_and(|old| {
                    Arc::ptr_eq(old.text, document.text) || old.text == document.text
                }) =>
            {
                previous.clone()
            }
            previous => {
                let next = analyze(document, previous, schema, control)?;
                control.checkpoint()?;
                if previous
                    .as_ref()
                    .is_none_or(|old| old.project_facts != next.project_facts)
                {
                    project_changed.insert((*key).clone());
                }
                next
            }
        };
        analyses.insert((*key).clone(), analysis);
    }
    project_changed.extend(
        old.keys()
            .filter(|key| !new_effective.contains_key(key))
            .cloned(),
    );
    Ok((analyses, project_changed))
}

fn analyze(
    document: &EffectiveDocument<'_>,
    previous: Option<&DocumentAnalysis>,
    schema: Option<&ProjectSchema>,
    control: &dyn WorkControl,
) -> Result<DocumentAnalysis, Interrupted> {
    if cfg!(test) {
        ANALYZE_COUNT.with(|count| count.set(count.get() + 1));
    }
    regions::analyze(document, previous, schema, control)
}

fn participation_for(recovery: SourceRecovery) -> ValidationParticipation {
    let complete = ValidationParticipation::all_complete();
    complete
        .with_ast_structure(completeness(recovery.ast_structure()))
        .with_block_definitions(completeness(recovery.block_definitions()))
        .with_block_references(completeness(recovery.block_references()))
        .with_stable_ids(completeness(recovery.stable_ids()))
        .with_metadata(completeness(recovery.metadata()))
        .with_condition_functions(completeness(recovery.condition_functions()))
        .with_effect_functions(completeness(recovery.effect_functions()))
        .with_inline_markup(completeness(recovery.inline_markup()))
}

fn completeness(is_complete: bool) -> crate::validation::ValidationCompleteness {
    if is_complete {
        crate::validation::ValidationCompleteness::Complete
    } else {
        crate::validation::ValidationCompleteness::Incomplete
    }
}

pub(super) fn build_documents(
    effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    analyses: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    semantic: &BTreeMap<recite_core::DocumentKey, Vec<Diagnostic>>,
    old_snapshot: &AuthoringSnapshot,
    project_changed: &BTreeSet<recite_core::DocumentKey>,
    control: &dyn WorkControl,
) -> Result<Vec<DocumentSnapshot>, Interrupted> {
    let mut documents = Vec::with_capacity(effective.len());
    for (key, document) in effective {
        control.checkpoint()?;
        let Some(analysis) = analyses.get(*key) else {
            continue;
        };
        let diagnostics = match old_snapshot.document(key) {
            Some(old)
                if !project_changed.contains(*key)
                    && std::ptr::eq(old.source_text(), analysis.source.source()) =>
            {
                Arc::clone(old.shared_diagnostics())
            }
            _ => {
                let mut diagnostics = analysis.parse_diagnostics.to_vec();
                diagnostics.extend(analysis.local_diagnostics.iter().cloned());
                if let Some(semantic) = semantic.get(*key) {
                    diagnostics.extend(semantic.iter().cloned());
                }
                sort_diagnostics_by_source(&mut diagnostics);
                diagnostics.into()
            }
        };
        documents.push(DocumentSnapshot::from_shared(
            metadata(
                (*key).clone(),
                document.layer,
                document.version,
                analysis.byte_len,
                analysis.line_count,
                analysis.participation,
            ),
            diagnostics,
            Arc::clone(&analysis.summary),
            Arc::clone(&analysis.source),
            analysis.source_fingerprint.clone(),
        ));
    }
    Ok(documents)
}

pub(super) fn build_delta(
    changed_keys: BTreeSet<recite_core::DocumentKey>,
    old_snapshot: &AuthoringSnapshot,
    new_documents: &[DocumentSnapshot],
) -> (Vec<DocumentDelta>, Vec<DocumentDelta>) {
    let mut changed = Vec::new();
    let mut removed = Vec::new();
    for key in changed_keys {
        let previous = old_snapshot
            .document(&key)
            .map(|document| document.metadata().clone());
        let current = new_documents
            .binary_search_by(|document| document.key().cmp(&key))
            .ok()
            .map(|index| new_documents[index].metadata().clone());
        let document_delta = delta(key, previous, current.clone());
        if current.is_some() {
            changed.push(document_delta);
        } else {
            removed.push(document_delta);
        }
    }
    (changed, removed)
}

#[cfg(test)]
mod tests;
