mod candidate;
mod field;
mod input;
mod label;
mod literal;
mod output;
mod query;
mod reference;
mod selector;

use std::collections::BTreeSet;

use self::input::lower_inputs;
use self::output::lower_outputs;
use self::query::lower_projector_queries;
use self::selector::lower_selector;
use super::super::raw::{
    Named, RawPresentationProjectorDefinition, RawProjectionQueryFunctionDefinition,
};
use super::super::spans::ManifestSpans;
use super::super::validate::{PendingTypeReference, duplicate_definition, validate_manifest_name};
use crate::Diagnostic;
use crate::schema::{ProjectSchema, SchemaPresentationProjectorDefinition};

pub(super) fn lower_projection_queries(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawProjectionQueryFunctionDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    query::lower_projection_queries(
        file,
        source,
        spans,
        entries,
        schema,
        diagnostics,
        pending_type_refs,
    );
}

pub(super) fn lower_presentation_projectors(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawPresentationProjectorDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    let mut seen_label_ids = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["presentation_projectors".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "projector id", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(
                diagnostics,
                "presentation projector",
                &entry.name,
                name_span,
            );
            continue;
        }

        let Some(candidates) = lower_selector(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            entry.value.candidates,
            &entry_path,
        ) else {
            continue;
        };
        let inputs = lower_inputs(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &candidates,
            entry.value.inputs,
            pending_type_refs,
            &entry_path,
        );
        let queries = lower_projector_queries(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &inputs,
            entry.value.queries,
            &entry_path,
        );
        let outputs = lower_outputs(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &inputs,
            &queries,
            entry.value.outputs,
            &mut seen_label_ids,
            pending_type_refs,
            &entry_path,
        );

        schema.presentation_projectors.insert(
            entry.name,
            SchemaPresentationProjectorDefinition {
                candidates,
                inputs,
                queries,
                outputs,
            },
        );
    }
}
