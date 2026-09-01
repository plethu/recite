use std::collections::BTreeMap;

use recite_core::{
    AvailabilityReasonArgBinding, ConditionReturnType, MetadataContextSelector,
    MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy, SchemaLiteralValue,
    SchemaTypeDefinition, SchemaTypeRef,
};

use super::super::fingerprints::producer_fingerprint_projection;
use super::super::provenance::origin_json;

pub(crate) fn json_type_definition(definition: &SchemaTypeDefinition) -> serde_json::Value {
    match definition {
        SchemaTypeDefinition::Enum(definition) => serde_json::json!({
            "kind": "enum",
            "values": definition.values.iter().collect::<Vec<_>>(),
        }),
    }
}

pub(crate) fn json_registry_definition(
    definition: &recite_core::RegistryDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "values": definition.values.iter().collect::<Vec<_>>(),
        "origin": definition.origin.as_ref().map(origin_json),
        "value_origins": definition.value_origins.iter().map(|(name, origin)| (name.clone(), origin_json(origin))).collect::<BTreeMap<_, _>>(),
        "producer_fingerprints": definition.producer_fingerprints.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
    })
}

pub(crate) fn json_condition_definition(
    definition: &recite_core::ConditionDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "params": definition.params.iter().map(json_parameter).collect::<Vec<_>>(),
        "returns": json_condition_return(&definition.returns),
        "availability_reason": definition.availability_reason.as_ref().map(|mapping| serde_json::json!({
            "reason": mapping.reason.as_str(),
            "args": mapping.args.iter().map(|(name, value)| (name.clone(), json_reason_binding(value))).collect::<BTreeMap<_, _>>(),
        })),
    })
}

pub(crate) fn json_condition_return(value: &ConditionReturnType) -> String {
    match value {
        ConditionReturnType::Bool => "bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("enum:{name}"),
    }
}

pub(crate) fn json_availability_reason_definition(
    definition: &recite_core::AvailabilityReasonDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "template": definition.template,
        "params": definition.params.iter().map(json_parameter).collect::<Vec<_>>(),
        "origin": definition.origin.as_ref().map(origin_json),
    })
}

pub(crate) fn json_effect_definition(
    definition: &recite_core::EffectDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "modes": definition.modes.iter().map(effect_mode).collect::<Vec<_>>(),
        "params": definition.params.iter().map(json_parameter).collect::<Vec<_>>(),
    })
}

pub(crate) fn effect_mode(mode: &recite_core::EffectMode) -> &'static str {
    match mode {
        recite_core::EffectMode::Deferred => "deferred",
        recite_core::EffectMode::Immediate => "immediate",
        recite_core::EffectMode::Blocking => "blocking",
    }
}

pub(crate) fn json_metadata_domain_definition(
    definition: &MetadataDomainDefinition,
) -> serde_json::Value {
    match definition {
        MetadataDomainDefinition::Flat(domain) => serde_json::json!({
            "kind": "flat",
            "values": domain.values.iter().collect::<Vec<_>>(),
            "origin": domain.provenance.origin.as_ref().map(origin_json),
            "value_origins": domain.provenance.value_origins.iter().map(|(name, origin)| (name.clone(), origin_json(origin))).collect::<BTreeMap<_, _>>(),
            "producer_fingerprints": domain.provenance.producer_fingerprints.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
        }),
        MetadataDomainDefinition::Contextual(domain) => serde_json::json!({
            "kind": "contextual",
            "selector": json_selector(&domain.selector),
            "values_by_context": domain.values_by_context.iter().map(|(name, values)| (name.clone(), values.iter().collect::<Vec<_>>())).collect::<BTreeMap<_, _>>(),
            "missing_context": json_missing_context(&domain.missing_context),
            "origin": domain.provenance.origin.as_ref().map(origin_json),
            "context_origins": domain.provenance.context_origins.iter().map(|(name, origin)| (name.clone(), origin_json(origin))).collect::<BTreeMap<_, _>>(),
            "value_origins": domain.provenance.value_origins.iter().map(|(context, values)| (context.clone(), values.iter().map(|(name, origin)| (name.clone(), origin_json(origin))).collect::<BTreeMap<_, _>>())).collect::<BTreeMap<_, _>>(),
            "producer_fingerprints": domain.provenance.producer_fingerprints.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
        }),
    }
}

pub(crate) fn json_selector(value: &MetadataContextSelector) -> String {
    match value {
        MetadataContextSelector::FieldSpeaker => "field:speaker".to_owned(),
        MetadataContextSelector::MetadataKey(name) => format!("metadata:{name}"),
    }
}

pub(crate) fn json_missing_context(value: &MissingMetadataContextPolicy) -> serde_json::Value {
    match value {
        MissingMetadataContextPolicy::Diagnostic => serde_json::json!({ "policy": "diagnostic" }),
        MissingMetadataContextPolicy::Empty => serde_json::json!({ "policy": "empty" }),
        MissingMetadataContextPolicy::Fallback { domain } => {
            serde_json::json!({ "policy": "fallback", "domain": domain })
        }
    }
}

pub(crate) fn json_metadata_definition(
    definition: &recite_core::MetadataDefinition,
) -> serde_json::Value {
    serde_json::json!({
        "targets": definition.targets.iter().map(metadata_target).collect::<Vec<_>>(),
        "type": json_type_ref(&definition.type_ref),
        "repeatable": definition.repeatable,
        "domain": definition.domain,
    })
}

pub(crate) fn metadata_target(value: &MetadataTarget) -> &'static str {
    match value {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}

pub(crate) fn json_parameter(value: &recite_core::ParameterDefinition) -> serde_json::Value {
    serde_json::json!({ "name": value.name, "type": json_type_ref(&value.type_ref) })
}

pub(crate) fn json_type_ref(value: &SchemaTypeRef) -> String {
    match value {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(value) => format!("array:{}", json_type_ref(value)),
    }
}

pub(crate) fn json_reason_binding(value: &AvailabilityReasonArgBinding) -> serde_json::Value {
    match value {
        AvailabilityReasonArgBinding::ConditionParam(name) => {
            serde_json::json!({ "kind": "condition_param", "name": name })
        }
        AvailabilityReasonArgBinding::Literal(value) => json_literal(value),
    }
}

pub(crate) fn json_literal(value: &SchemaLiteralValue) -> serde_json::Value {
    match value {
        SchemaLiteralValue::String(value) => {
            serde_json::json!({ "kind": "literal", "type": "string", "value": value })
        }
        SchemaLiteralValue::Int(value) => {
            serde_json::json!({ "kind": "literal", "type": "int", "value": value })
        }
        SchemaLiteralValue::Float(value) => {
            serde_json::json!({ "kind": "literal", "type": "float", "value": value })
        }
        SchemaLiteralValue::Bool(value) => {
            serde_json::json!({ "kind": "literal", "type": "bool", "value": value })
        }
    }
}
