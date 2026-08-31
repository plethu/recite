use std::collections::BTreeMap;

use recite_core::{
    MetadataOccurrence, ProjectionInputRef, ProjectionOutputTarget, ProjectionQueryDefinition,
    SchemaProjectionInputSource, SchemaProjectionSelector,
};

use super::core::{json_literal, json_parameter, json_type_ref, metadata_target};

pub(crate) fn json_projection_query_function(
    definition: &recite_core::ProjectionQueryFunctionDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "params": definition.params.iter().map(json_parameter).collect::<Vec<_>>(),
        "returns": json_type_ref(&definition.returns),
        "max_calls_per_event": definition.max_calls_per_event,
    })
}

pub(crate) fn json_presentation_projector(
    definition: &recite_core::SchemaPresentationProjectorDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "candidates": json_projection_selector(&definition.candidates),
        "inputs": definition.inputs.iter().map(json_projection_input).collect::<Vec<_>>(),
        "queries": definition.queries.iter().map(|(name, query)| (name.clone(), json_projection_query(query))).collect::<BTreeMap<_, _>>(),
        "outputs": definition.outputs.iter().map(|(name, output)| (name.clone(), serde_json::json!({
            "target": json_output_target(&output.target),
            "kind": output.kind,
            "slot": output.slot,
            "label": output.label.as_ref().map(json_label),
            "fields": output.fields.iter().map(|(name, field)| (name.clone(), serde_json::json!({ "source": json_field_source(&field.source), "type": json_type_ref(&field.type_ref) }))).collect::<BTreeMap<_, _>>(),
        }))).collect::<BTreeMap<_, _>>(),
    })
}

pub(crate) fn json_projection_selector(value: &SchemaProjectionSelector) -> serde_json::Value {
    match value {
        SchemaProjectionSelector::RuntimeEvent { kind } => {
            serde_json::json!({ "kind": "runtime_event", "event": kind })
        }
        SchemaProjectionSelector::MetadataKey { target, key } => {
            serde_json::json!({ "kind": "metadata_key", "target": metadata_target(target), "key": key })
        }
        SchemaProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => serde_json::json!({
            "kind": "metadata_set",
            "target": metadata_target(target),
            "required_keys": required_keys,
        }),
        SchemaProjectionSelector::AvailabilityReason { reason_id } => {
            serde_json::json!({ "kind": "availability_reason", "reason": reason_id.as_str() })
        }
        _ => serde_json::json!({ "kind": "unknown" }),
    }
}

pub(crate) fn json_projection_input(value: &recite_core::ProjectionInput) -> serde_json::Value {
    serde_json::json!({
        "name": value.name,
        "source": json_input_source(&value.source),
        "type": json_type_ref(&value.type_ref),
        "required": value.required,
    })
}

pub(crate) fn json_input_source(value: &SchemaProjectionInputSource) -> serde_json::Value {
    match value {
        SchemaProjectionInputSource::EventKind => serde_json::json!({ "kind": "event_kind" }),
        SchemaProjectionInputSource::CandidateLineId => {
            serde_json::json!({ "kind": "candidate_line_id" })
        }
        SchemaProjectionInputSource::CandidateChoiceId => {
            serde_json::json!({ "kind": "candidate_choice_id" })
        }
        SchemaProjectionInputSource::CandidateEffectRequestId => {
            serde_json::json!({ "kind": "candidate_effect_request_id" })
        }
        SchemaProjectionInputSource::CandidateBlockId => {
            serde_json::json!({ "kind": "candidate_block_id" })
        }
        SchemaProjectionInputSource::CandidateProject => {
            serde_json::json!({ "kind": "candidate_project" })
        }
        SchemaProjectionInputSource::CandidateMetadata { key, occurrence } => {
            serde_json::json!({ "kind": "candidate_metadata", "key": key, "occurrence": json_occurrence(occurrence) })
        }
        SchemaProjectionInputSource::AvailabilityReasonArg { name } => {
            serde_json::json!({ "kind": "availability_reason_arg", "name": name })
        }
        SchemaProjectionInputSource::Literal(value) => json_literal(value),
        _ => serde_json::json!({ "kind": "unknown" }),
    }
}

pub(crate) fn json_occurrence(value: &MetadataOccurrence) -> serde_json::Value {
    match value {
        MetadataOccurrence::Only => serde_json::json!({ "kind": "only" }),
        MetadataOccurrence::First => serde_json::json!({ "kind": "first" }),
        MetadataOccurrence::Last => serde_json::json!({ "kind": "last" }),
        MetadataOccurrence::All => serde_json::json!({ "kind": "all" }),
        MetadataOccurrence::Index(index) => serde_json::json!({ "kind": "index", "index": index }),
        _ => serde_json::json!({ "kind": "unknown" }),
    }
}

pub(crate) fn json_projection_query(value: &ProjectionQueryDefinition) -> serde_json::Value {
    serde_json::json!({
        "function": value.function,
        "args": value.args.iter().map(json_input_ref).collect::<Vec<_>>(),
    })
}

pub(crate) fn json_input_ref(value: &ProjectionInputRef) -> serde_json::Value {
    match value {
        ProjectionInputRef::Input { name } => {
            serde_json::json!({ "kind": "input", "name": name })
        }
        ProjectionInputRef::QueryResult { name } => {
            serde_json::json!({ "kind": "query_result", "name": name })
        }
        _ => serde_json::json!({ "kind": "unknown" }),
    }
}

pub(crate) fn json_output_target(value: &ProjectionOutputTarget) -> &'static str {
    match value {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
        _ => "unknown",
    }
}

pub(crate) fn json_label(value: &recite_core::PresentationLabelDefinition) -> serde_json::Value {
    serde_json::json!({
        "template_id": value.template_id,
        "source_text": value.source_text,
        "args": value.args.iter().map(|(name, arg)| (name.clone(), serde_json::json!({ "source": json_input_ref(&arg.source), "type": json_type_ref(&arg.type_ref) }))).collect::<BTreeMap<_, _>>(),
    })
}

pub(crate) fn json_field_source(
    value: &recite_core::PresentationAffordanceFieldSource,
) -> serde_json::Value {
    match value {
        recite_core::PresentationAffordanceFieldSource::Input { name } => {
            serde_json::json!({ "kind": "input", "name": name })
        }
        recite_core::PresentationAffordanceFieldSource::QueryResult { name } => {
            serde_json::json!({ "kind": "query_result", "name": name })
        }
        recite_core::PresentationAffordanceFieldSource::Literal(value) => json_literal(value),
        _ => serde_json::json!({ "kind": "unknown" }),
    }
}
