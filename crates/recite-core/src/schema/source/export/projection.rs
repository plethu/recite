use std::collections::BTreeMap;

use super::{basic::type_ref_name, insert_object};
use crate::schema::{
    MetadataOccurrence, PresentationAffordanceFieldSource, PresentationAffordanceOutputDefinition,
    ProjectSchema, ProjectionInput, ProjectionInputRef, ProjectionOutputTarget,
    SchemaProjectionInputSource, SchemaProjectionSelector,
};

pub(super) fn insert_sections(
    root: &mut serde_json::Map<String, serde_json::Value>,
    schema: &ProjectSchema,
) {
    let mut projection_queries = serde_json::Map::new();
    for (name, definition) in &schema.projection_queries {
        let mut value = serde_json::Map::new();
        value.insert("params".to_owned(), json_params(&definition.params));
        value.insert(
            "returns".to_owned(),
            serde_json::json!(type_ref_name(&definition.returns)),
        );
        if let Some(max) = definition.max_calls_per_event {
            value.insert("max_calls_per_event".to_owned(), serde_json::json!(max));
        }
        projection_queries.insert(name.clone(), serde_json::Value::Object(value));
    }
    insert_object(root, "projection_queries", projection_queries);

    let mut projectors = serde_json::Map::new();
    for (name, definition) in &schema.presentation_projectors {
        let mut value = serde_json::Map::new();
        value.insert(
            "candidates".to_owned(),
            json_selector(&definition.candidates),
        );
        value.insert(
            "inputs".to_owned(),
            serde_json::Value::Array(definition.inputs.iter().map(json_input).collect()),
        );
        let mut queries = serde_json::Map::new();
        for (query_name, query) in &definition.queries {
            queries.insert(
                query_name.clone(),
                serde_json::json!({
                    "function": query.function,
                    "args": query.args.iter().map(json_input_ref).collect::<Vec<_>>()
                }),
            );
        }
        value.insert("queries".to_owned(), serde_json::Value::Object(queries));
        let mut outputs = serde_json::Map::new();
        for (output_name, output) in &definition.outputs {
            outputs.insert(output_name.clone(), json_output(output));
        }
        value.insert("outputs".to_owned(), serde_json::Value::Object(outputs));
        projectors.insert(name.clone(), serde_json::Value::Object(value));
    }
    insert_object(root, "presentation_projectors", projectors);
}

fn json_params(params: &[crate::schema::ParameterDefinition]) -> serde_json::Value {
    serde_json::Value::Array(
        params
            .iter()
            .map(|param| serde_json::json!({ "name": param.name, "type": type_ref_name(&param.type_ref) }))
            .collect(),
    )
}

fn json_selector(selector: &SchemaProjectionSelector) -> serde_json::Value {
    match selector {
        SchemaProjectionSelector::RuntimeEvent { kind } => {
            serde_json::json!({ "kind": "runtime_event", "event": kind })
        }
        SchemaProjectionSelector::MetadataKey { target, key } => serde_json::json!({
            "kind": "metadata_key", "target": metadata_target_name(target), "key": key
        }),
        SchemaProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => serde_json::json!({
            "kind": "metadata_set", "target": metadata_target_name(target),
            "required_keys": required_keys
        }),
        SchemaProjectionSelector::AvailabilityReason { reason_id } => {
            serde_json::json!({ "kind": "availability_reason", "reason": reason_id.as_str() })
        }
    }
}

fn json_input(input: &ProjectionInput) -> serde_json::Value {
    serde_json::json!({
        "name": input.name,
        "source": json_input_source(&input.source),
        "type": type_ref_name(&input.type_ref),
        "required": input.required
    })
}

fn json_input_source(source: &SchemaProjectionInputSource) -> serde_json::Value {
    match source {
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
        SchemaProjectionInputSource::CandidateMetadata { key, occurrence } => serde_json::json!({
            "kind": "candidate_metadata", "key": key, "occurrence": json_occurrence(occurrence)
        }),
        SchemaProjectionInputSource::AvailabilityReasonArg { name } => {
            serde_json::json!({ "kind": "availability_reason_arg", "name": name })
        }
        SchemaProjectionInputSource::Literal(value) => {
            serde_json::json!({ "kind": "literal", "value": json_literal(value) })
        }
    }
}

fn json_literal(value: &crate::schema::SchemaLiteralValue) -> serde_json::Value {
    match value {
        crate::schema::SchemaLiteralValue::String(value) => {
            serde_json::json!(value)
        }
        crate::schema::SchemaLiteralValue::Int(value) => serde_json::json!(value),
        crate::schema::SchemaLiteralValue::Float(value) => serde_json::Number::from_str(value)
            .map_or_else(
                |_| serde_json::Value::String(value.clone()),
                serde_json::Value::Number,
            ),
        crate::schema::SchemaLiteralValue::Bool(value) => serde_json::json!(value),
    }
}

fn json_occurrence(occurrence: &MetadataOccurrence) -> serde_json::Value {
    match occurrence {
        MetadataOccurrence::Only => serde_json::json!("only"),
        MetadataOccurrence::First => serde_json::json!("first"),
        MetadataOccurrence::Last => serde_json::json!("last"),
        MetadataOccurrence::All => serde_json::json!("all"),
        MetadataOccurrence::Index(index) => serde_json::json!({ "index": index }),
    }
}

fn json_input_ref(input_ref: &ProjectionInputRef) -> serde_json::Value {
    match input_ref {
        ProjectionInputRef::Input { name } => serde_json::json!({ "input": name }),
        ProjectionInputRef::QueryResult { name } => serde_json::json!({ "query_result": name }),
    }
}

fn json_output(output: &PresentationAffordanceOutputDefinition) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    value.insert(
        "target".to_owned(),
        serde_json::json!(output_target_name(&output.target)),
    );
    value.insert("kind".to_owned(), serde_json::json!(output.kind));
    value.insert("slot".to_owned(), serde_json::json!(output.slot));
    if let Some(label) = &output.label {
        let args = label
            .args
            .iter()
            .map(|(name, arg)| (
                name.clone(),
                serde_json::json!({ "source": json_input_ref(&arg.source), "type": type_ref_name(&arg.type_ref) }),
            ))
            .collect::<BTreeMap<_, _>>();
        value.insert(
            "label".to_owned(),
            serde_json::json!({
                "template_id": label.template_id,
                "source_text": label.source_text,
                "args": args
            }),
        );
    }
    let fields = output
        .fields
        .iter()
        .map(|(name, field)| (
            name.clone(),
            serde_json::json!({ "source": json_field_source(&field.source), "type": type_ref_name(&field.type_ref) }),
        ))
        .collect::<BTreeMap<_, _>>();
    value.insert("fields".to_owned(), serde_json::json!(fields));
    serde_json::Value::Object(value)
}

fn json_field_source(source: &PresentationAffordanceFieldSource) -> serde_json::Value {
    match source {
        PresentationAffordanceFieldSource::Input { name } => {
            serde_json::json!({ "kind": "input", "name": name })
        }
        PresentationAffordanceFieldSource::QueryResult { name } => {
            serde_json::json!({ "kind": "query_result", "name": name })
        }
        PresentationAffordanceFieldSource::Literal(value) => {
            serde_json::json!({ "kind": "literal", "value": json_literal(value) })
        }
    }
}

fn metadata_target_name(target: &crate::schema::MetadataTarget) -> &'static str {
    match target {
        crate::schema::MetadataTarget::Block => "block",
        crate::schema::MetadataTarget::Choice => "choice",
        crate::schema::MetadataTarget::Line => "line",
        crate::schema::MetadataTarget::Project => "project",
    }
}

fn output_target_name(target: &ProjectionOutputTarget) -> &'static str {
    match target {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
    }
}
use std::str::FromStr;
