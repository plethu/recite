mod candidate;
mod field;
mod input;
mod label;
mod literal;
mod output;
mod query;
mod reference;
mod selector;

use std::collections::{BTreeMap, BTreeSet};

use self::input::lower_inputs;
use self::output::lower_outputs;
use self::query::lower_projector_queries;
use self::selector::lower_selector;
use super::super::raw::{
    Named, RawPresentationProjectorDefinition, RawProjectionQueryFunctionDefinition,
};
use super::super::spans::ManifestSpans;
use super::super::validate::{PendingTypeReference, duplicate_definition, validate_manifest_name};
use super::LoweringContext;
use crate::Diagnostic;
use crate::schema::{
    ParameterDefinition, ProjectSchema, SchemaPresentationProjectorDefinition,
    SchemaProjectionSelector, SchemaTypeRef,
};

pub(super) struct ProjectorContext<'a> {
    pub(super) schema: &'a ProjectSchema,
    pub(super) projector: &'a str,
}

pub(super) struct ProjectionTypeTables {
    pub(super) input_types: BTreeMap<String, SchemaTypeRef>,
    pub(super) query_types: BTreeMap<String, SchemaTypeRef>,
}

pub(super) struct InputSourceContext<'a> {
    pub(super) schema: &'a ProjectSchema,
    pub(super) projector: &'a str,
    pub(super) selector: &'a SchemaProjectionSelector,
    pub(super) input: &'a str,
    pub(super) type_ref: &'a SchemaTypeRef,
}

pub(super) struct CandidateMetadataContext<'a> {
    pub(super) schema: &'a ProjectSchema,
    pub(super) projector: &'a str,
    pub(super) input: &'a str,
    pub(super) selector: &'a SchemaProjectionSelector,
    pub(super) key: &'a str,
    pub(super) occurrence: &'a crate::schema::MetadataOccurrence,
    pub(super) type_ref: &'a SchemaTypeRef,
}

pub(super) struct AvailabilityReasonContext<'a> {
    pub(super) schema: &'a ProjectSchema,
    pub(super) projector: &'a str,
    pub(super) input: &'a str,
    pub(super) selector: &'a SchemaProjectionSelector,
    pub(super) name: &'a str,
    pub(super) type_ref: &'a SchemaTypeRef,
}

pub(super) struct ReferenceTypeContext<'a, T> {
    pub(super) projector: &'a str,
    pub(super) owner: &'a str,
    pub(super) expected: &'a SchemaTypeRef,
    pub(super) input_types: &'a BTreeMap<String, T>,
    pub(super) query_types: &'a BTreeMap<String, SchemaTypeRef>,
}

pub(super) struct QueryArgumentContext<'a> {
    pub(super) projector: &'a str,
    pub(super) query: &'a str,
    pub(super) function: &'a str,
    pub(super) params: &'a [ParameterDefinition],
    pub(super) input_types: &'a BTreeMap<String, SchemaTypeRef>,
    pub(super) query_types: &'a BTreeMap<String, SchemaTypeRef>,
}

pub(super) struct ProjectionBinding<'a> {
    pub(super) projector: &'a str,
    pub(super) output: &'a str,
    pub(super) name: &'a str,
}

pub(super) struct FieldSpans {
    pub(super) span: crate::SourceSpan,
    pub(super) literal_span: crate::SourceSpan,
}

pub(super) struct FieldSourceContext<'a> {
    pub(super) schema: &'a ProjectSchema,
    pub(super) projector: &'a str,
    pub(super) output: &'a str,
    pub(super) field: &'a str,
    pub(super) type_ref: &'a SchemaTypeRef,
    pub(super) types: &'a ProjectionTypeTables,
    pub(super) spans: FieldSpans,
}

pub(super) struct LabelLoweringState<'a> {
    pub(super) seen_label_ids: &'a mut BTreeSet<String>,
    pub(super) pending_type_refs: &'a mut Vec<super::super::validate::PendingTypeReference>,
}

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
    let mut lowering = LoweringContext::new(file, source, spans, diagnostics);
    for entry in entries {
        let entry_path = vec!["presentation_projectors".to_owned(), entry.name.clone()];
        let name_span = lowering.key_span_at(&entry_path, &entry.name);
        if !validate_manifest_name(
            lowering.diagnostics,
            "projector id",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(
                lowering.diagnostics,
                "presentation projector",
                &entry.name,
                name_span,
            );
            continue;
        }

        let projector_context = ProjectorContext {
            schema,
            projector: &entry.name,
        };
        let Some(candidates) = lower_selector(
            &mut lowering,
            projector_context.schema,
            projector_context.projector,
            entry.value.candidates,
            &entry_path,
        ) else {
            continue;
        };
        let inputs = lower_inputs(
            &mut lowering,
            projector_context.schema,
            projector_context.projector,
            &candidates,
            entry.value.inputs,
            pending_type_refs,
            &entry_path,
        );
        let queries = lower_projector_queries(
            &mut lowering,
            projector_context.schema,
            projector_context.projector,
            &inputs,
            entry.value.queries,
            &entry_path,
        );
        let mut state = LabelLoweringState {
            seen_label_ids: &mut seen_label_ids,
            pending_type_refs,
        };
        let outputs = lower_outputs(
            &mut lowering,
            projector_context,
            &inputs,
            &queries,
            entry.value.outputs,
            &mut state,
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
