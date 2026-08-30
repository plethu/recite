use std::collections::{BTreeMap, BTreeSet};

use recite_core::{Diagnostic, ProjectSchema};
use recite_parser::parse;

use super::AuthoringSummary;
use super::input_state::EffectiveDocument;
use super::snapshot::{AuthoringSnapshot, DocumentDelta, DocumentSnapshot, delta, metadata};
use super::state::DocumentAnalysis;
use crate::validation::{
    project::sort_diagnostics_by_source, validate_source_files_with_participation,
    validate_source_files_with_participation_with_schema,
};
use crate::{ValidationInput, ValidationParticipation};

pub(super) fn rebuild_analyses(
    mut old: BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    old_effective: &BTreeMap<recite_core::DocumentKey, EffectiveDocument>,
    new_effective: &BTreeMap<recite_core::DocumentKey, EffectiveDocument>,
) -> BTreeMap<recite_core::DocumentKey, DocumentAnalysis> {
    new_effective
        .iter()
        .map(|(key, document)| {
            let analysis = if old_effective
                .get(key)
                .is_some_and(|old_document| old_document.text == document.text)
            {
                match old.remove(key) {
                    Some(analysis) => analysis,
                    None => analyze(document),
                }
            } else {
                analyze(document)
            };
            (key.clone(), analysis)
        })
        .collect()
}

fn analyze(document: &EffectiveDocument) -> DocumentAnalysis {
    let parsed = parse(document.key.as_str(), document.text.clone());
    let lowered = parsed.lower_source_file();
    let participation = if lowered.diagnostics.is_empty() {
        ValidationParticipation::all_complete()
    } else {
        ValidationParticipation::all_incomplete()
    };
    let summary = AuthoringSummary::from_source_file(&lowered.source_file);
    DocumentAnalysis {
        text: document.text.clone(),
        source_file: lowered.source_file,
        parse_diagnostics: lowered.diagnostics,
        summary,
        participation,
    }
}

pub(super) fn validate_analyses(
    analyses: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    schema: Option<&ProjectSchema>,
) -> BTreeMap<recite_core::DocumentKey, Vec<Diagnostic>> {
    let inputs = analyses
        .values()
        .map(|analysis| ValidationInput::new(&analysis.source_file, analysis.participation))
        .collect::<Vec<_>>();
    let report = schema.map_or_else(
        || validate_source_files_with_participation(&inputs),
        |schema| validate_source_files_with_participation_with_schema(&inputs, schema),
    );
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
    effective: &BTreeMap<recite_core::DocumentKey, EffectiveDocument>,
    analyses: &BTreeMap<recite_core::DocumentKey, DocumentAnalysis>,
    semantic: &BTreeMap<recite_core::DocumentKey, Vec<Diagnostic>>,
) -> Vec<DocumentSnapshot> {
    effective
        .iter()
        .filter_map(|(key, document)| {
            let analysis = analyses.get(key)?;
            let mut diagnostics = analysis.parse_diagnostics.clone();
            if let Some(semantic) = semantic.get(key) {
                diagnostics.extend(semantic.iter().cloned());
            }
            sort_diagnostics_by_source(&mut diagnostics);
            Some(DocumentSnapshot::new(
                metadata(
                    key.clone(),
                    document.layer,
                    document.version,
                    &analysis.text,
                    analysis.participation,
                ),
                diagnostics,
                analysis.summary.clone(),
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
