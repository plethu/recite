use super::{insert_object, json_literal_string, provenance};
use crate::EffectMode;
use crate::schema::{
    AvailabilityReasonArgBinding, ConditionReturnType, MetadataContextSelector,
    MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy, ParameterDefinition,
    ProjectSchema, SchemaLiteralValue, SchemaTypeDefinition,
};

pub(super) fn insert_sections(
    root: &mut serde_json::Map<String, serde_json::Value>,
    schema: &ProjectSchema,
) {
    use serde_json::{Map, Value, json};

    let mut types = Map::new();
    for (name, definition) in &schema.types {
        match definition {
            SchemaTypeDefinition::Enum(definition) => {
                types.insert(
                    name.clone(),
                    json!({
                        "kind": "enum",
                        "values": definition.values.iter().collect::<Vec<_>>()
                    }),
                );
            }
        }
    }
    insert_object(root, "types", types);

    let mut registries = Map::new();
    for (name, definition) in &schema.registries {
        let mut value = serde_json::Map::new();
        value.insert(
            "values".to_owned(),
            json!(definition.values.iter().collect::<Vec<_>>()),
        );
        provenance::add_origin(&mut value, definition.origin.as_ref());
        if !definition.value_origins.is_empty() {
            value.insert(
                "value_origins".to_owned(),
                provenance::json_origin_map(&definition.value_origins),
            );
        }
        if !definition.producer_fingerprints.is_empty() {
            value.insert(
                "producer_fingerprints".to_owned(),
                provenance::json_fingerprints(&definition.producer_fingerprints),
            );
        }
        registries.insert(name.clone(), serde_json::Value::Object(value));
    }
    insert_object(root, "registries", registries);

    let mut speakers = Map::new();
    for (name, definition) in &schema.speakers {
        let mut value = Map::new();
        if let Some(display_name) = &definition.display_name {
            value.insert("display_name".to_owned(), json!(display_name));
        }
        speakers.insert(name.clone(), Value::Object(value));
    }
    insert_object(root, "speakers", speakers);

    let mut conditions = Map::new();
    for (name, definition) in &schema.conditions {
        let mut value = Map::new();
        value.insert("params".to_owned(), json_params(&definition.params));
        value.insert(
            "returns".to_owned(),
            json!(match &definition.returns {
                ConditionReturnType::Bool => "bool".to_owned(),
                ConditionReturnType::Enum(name) => format!("enum:{name}"),
            }),
        );
        if let Some(mapping) = &definition.availability_reason {
            let mut args = Map::new();
            for (name, binding) in &mapping.args {
                args.insert(name.clone(), json_binding(binding));
            }
            value.insert(
                "availability_reason".to_owned(),
                json!({ "reason": mapping.reason.as_str(), "args": args }),
            );
        }
        conditions.insert(name.clone(), Value::Object(value));
    }
    insert_object(root, "conditions", conditions);

    let mut reasons = Map::new();
    for (id, reason) in &schema.availability_reasons {
        let mut value = serde_json::Map::new();
        value.insert("template".to_owned(), json!(reason.template));
        value.insert("params".to_owned(), json_params(&reason.params));
        provenance::add_origin(&mut value, reason.origin.as_ref());
        reasons.insert(id.as_str().to_owned(), serde_json::Value::Object(value));
    }
    insert_object(root, "availability_reasons", reasons);

    let mut effects = Map::new();
    for (name, definition) in &schema.effects {
        effects.insert(
            name.clone(),
            json!({
                "modes": definition.modes.iter().map(effect_mode_name).collect::<Vec<_>>(),
                "params": json_params(&definition.params)
            }),
        );
    }
    insert_object(root, "effects", effects);

    let mut domains = Map::new();
    for (name, definition) in &schema.metadata_domains {
        domains.insert(name.clone(), json_domain(definition));
    }
    insert_object(root, "metadata_domains", domains);

    let mut metadata = Map::new();
    for (name, definition) in &schema.metadata {
        let mut value = Map::new();
        value.insert(
            "targets".to_owned(),
            json!(
                definition
                    .targets
                    .iter()
                    .map(metadata_target_name)
                    .collect::<Vec<_>>()
            ),
        );
        value.insert(
            "type".to_owned(),
            json!(type_ref_name(&definition.type_ref)),
        );
        value.insert("repeatable".to_owned(), json!(definition.repeatable));
        if let Some(domain) = &definition.domain {
            value.insert("domain".to_owned(), json!(domain));
        }
        metadata.insert(name.clone(), Value::Object(value));
    }
    insert_object(root, "metadata", metadata);

    let mut markup = Map::new();
    for (name, definition) in &schema.markup {
        markup.insert(
            name.clone(),
            json!({
                "requires_closing": definition.requires_closing,
                "translatable": definition.translatable,
                "allows_nesting": definition.allows_nesting
            }),
        );
    }
    insert_object(root, "markup", markup);
}

fn json_params(params: &[ParameterDefinition]) -> serde_json::Value {
    serde_json::Value::Array(
        params
            .iter()
            .map(|param| {
                serde_json::json!({
                    "name": param.name,
                    "type": type_ref_name(&param.type_ref)
                })
            })
            .collect(),
    )
}

fn json_binding(binding: &AvailabilityReasonArgBinding) -> serde_json::Value {
    match binding {
        AvailabilityReasonArgBinding::ConditionParam(name) => {
            serde_json::Value::String(format!("${name}"))
        }
        AvailabilityReasonArgBinding::Literal(value) => json_literal(value),
    }
}

fn json_literal(value: &SchemaLiteralValue) -> serde_json::Value {
    match value {
        SchemaLiteralValue::String(value) => serde_json::Value::String(json_literal_string(value)),
        SchemaLiteralValue::Int(value) => serde_json::json!(value),
        SchemaLiteralValue::Float(value) => serde_json::Number::from_str(value).map_or_else(
            |_| serde_json::Value::String(value.clone()),
            serde_json::Value::Number,
        ),
        SchemaLiteralValue::Bool(value) => serde_json::Value::Bool(*value),
    }
}

fn json_domain(domain: &MetadataDomainDefinition) -> serde_json::Value {
    match domain {
        MetadataDomainDefinition::Flat(domain) => {
            let mut value = serde_json::Map::new();
            value.insert("kind".to_owned(), serde_json::json!("flat"));
            value.insert(
                "values".to_owned(),
                serde_json::json!(domain.values.iter().collect::<Vec<_>>()),
            );
            provenance::add_flat_provenance(&mut value, &domain.provenance);
            serde_json::Value::Object(value)
        }
        MetadataDomainDefinition::Contextual(domain) => {
            let mut value = serde_json::Map::new();
            value.insert("kind".to_owned(), serde_json::json!("contextual"));
            value.insert(
                "selector".to_owned(),
                serde_json::json!(selector_name(&domain.selector)),
            );
            value.insert(
                "values_by_context".to_owned(),
                serde_json::Value::Object(
                    domain
                        .values_by_context
                        .iter()
                        .map(|(name, values)| {
                            (
                                name.clone(),
                                serde_json::json!(values.iter().collect::<Vec<_>>()),
                            )
                        })
                        .collect(),
                ),
            );
            value.insert(
                "missing_context".to_owned(),
                json_missing_context(&domain.missing_context),
            );
            provenance::add_contextual_provenance(&mut value, &domain.provenance);
            serde_json::Value::Object(value)
        }
    }
}

fn json_missing_context(policy: &MissingMetadataContextPolicy) -> serde_json::Value {
    match policy {
        MissingMetadataContextPolicy::Diagnostic => serde_json::json!({ "policy": "diagnostic" }),
        MissingMetadataContextPolicy::Empty => serde_json::json!({ "policy": "empty" }),
        MissingMetadataContextPolicy::Fallback { domain } => {
            serde_json::json!({ "policy": "fallback", "domain": domain })
        }
    }
}

fn effect_mode_name(mode: &EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}

fn metadata_target_name(target: &MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}

fn selector_name(selector: &MetadataContextSelector) -> String {
    match selector {
        MetadataContextSelector::FieldSpeaker => "field:speaker".to_owned(),
        MetadataContextSelector::MetadataKey(key) => format!("metadata:{key}"),
    }
}

pub(super) fn type_ref_name(type_ref: &crate::schema::SchemaTypeRef) -> String {
    use crate::schema::SchemaTypeRef;
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", type_ref_name(inner)),
    }
}
use std::str::FromStr;
