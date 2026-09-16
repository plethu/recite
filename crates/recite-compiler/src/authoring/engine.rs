use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recite_core::{Diagnostic, ProjectSchema, SourceRecovery};
use recite_parser::parse;

use super::AuthoringSummary;
use super::input_state::EffectiveDocument;
use super::snapshot::{AuthoringSnapshot, DocumentDelta, DocumentSnapshot, delta, metadata};
use super::state::DocumentAnalysis;
use crate::validation::{
    incremental::{same_project_inputs, validate_local, validate_project},
    project::sort_diagnostics_by_source,
};
use crate::{ValidationInput, ValidationParticipation};

use std::cell::Cell;

thread_local! {
    pub(super) static ANALYZE_COUNT: Cell<usize> = const { Cell::new(0) };
    pub(super) static PROJECT_VALIDATION_COUNT: Cell<usize> = const { Cell::new(0) };
}

pub(super) fn rebuild_analyses(
    mut old: BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    old_effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    new_effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    schema: Option<&ProjectSchema>,
) -> (BTreeMap<recite_core::DocumentKey, DocumentAnalysis>, bool) {
    let mut project_changed = old.len() != new_effective.len();
    let mut analyses = BTreeMap::new();
    for (key, document) in new_effective {
        let previous = old.remove(*key);
        let analysis = match previous {
            Some(previous)
                if old_effective.get(key).is_some_and(|old| {
                    std::ptr::eq(old.text, document.text) || old.text == document.text
                }) =>
            {
                previous
            }
            previous => {
                let next = analyze(document, schema);
                project_changed |= previous.as_ref().is_none_or(|old| {
                    !same_project_inputs(
                        ValidationInput::new(&old.source_file, old.participation),
                        ValidationInput::new(&next.source_file, next.participation),
                    )
                });
                next
            }
        };
        analyses.insert((*key).clone(), analysis);
    }
    (analyses, project_changed)
}

fn analyze(document: &EffectiveDocument<'_>, schema: Option<&ProjectSchema>) -> DocumentAnalysis {
    if cfg!(test) {
        ANALYZE_COUNT.with(|count| count.set(count.get() + 1));
    }
    let parsed = parse(document.key.as_str(), document.text);
    let lowered = parsed.lower_source_file();
    let participation = participation_for(lowered.recovery);
    let summary = AuthoringSummary::from_source_file(&lowered.source_file);
    let local_diagnostics = validate_local(
        ValidationInput::new(&lowered.source_file, participation),
        schema,
    )
    .diagnostics
    .into();
    DocumentAnalysis {
        local_diagnostics,
        source_file: lowered.source_file,
        source_text: Arc::from(document.text),
        parse_diagnostics: lowered.diagnostics.into(),
        summary: Arc::new(summary),
        participation,
        byte_len: document.text.len(),
        line_count: document.text.lines().count(),
    }
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

fn completeness(is_complete: bool) -> crate::ValidationCompleteness {
    if is_complete {
        crate::ValidationCompleteness::Complete
    } else {
        crate::ValidationCompleteness::Incomplete
    }
}

pub(super) fn validate_analyses(
    analyses: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    schema: Option<&ProjectSchema>,
    project_complete: bool,
) -> BTreeMap<recite_core::DocumentKey, Vec<Diagnostic>> {
    if cfg!(test) {
        PROJECT_VALIDATION_COUNT.with(|count| count.set(count.get() + 1));
    }
    let inputs = analyses
        .values()
        .map(|analysis| ValidationInput::new(&analysis.source_file, analysis.participation));
    let report = validate_project(inputs, schema, project_complete);
    let mut diagnostics = BTreeMap::<recite_core::DocumentKey, Vec<Diagnostic>>::new();
    for diagnostic in report.diagnostics {
        if let Ok(key) = recite_core::DocumentKey::new(diagnostic.span.file.clone())
            && analyses.contains_key(&key)
        {
            diagnostics.entry(key).or_default().push(diagnostic);
        }
    }
    diagnostics
}

pub(super) fn build_documents(
    effective: &BTreeMap<&recite_core::DocumentKey, EffectiveDocument<'_>>,
    analyses: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    semantic: &BTreeMap<recite_core::DocumentKey, Vec<Diagnostic>>,
    old_snapshot: &AuthoringSnapshot,
    project_changed: bool,
) -> Vec<DocumentSnapshot> {
    effective
        .iter()
        .filter_map(|(key, document)| {
            let analysis = analyses.get(*key)?;
            let diagnostics = match old_snapshot.document(key) {
                Some(old)
                    if !project_changed
                        && std::ptr::eq(old.source_text(), analysis.source_text.as_ref()) =>
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
            Some(DocumentSnapshot::from_shared(
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
                Arc::clone(&analysis.source_text),
            ))
        })
        .collect()
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
