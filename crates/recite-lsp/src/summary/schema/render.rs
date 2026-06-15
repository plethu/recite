use recite_core::{
    EffectMode, MetadataTarget, ProjectionOutputTarget, SchemaProjectionSelector, SchemaTypeRef,
};

pub(super) fn type_ref_summary(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", type_ref_summary(inner)),
    }
}

pub(super) fn metadata_target_name(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}

pub(super) fn projection_selector_summary(selector: &SchemaProjectionSelector) -> String {
    match selector {
        SchemaProjectionSelector::RuntimeEvent { kind } => format!("runtime_event:{kind}"),
        SchemaProjectionSelector::MetadataKey { target, key } => {
            format!("metadata_key:{}:{key}", metadata_target_name(*target))
        }
        SchemaProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => format!(
            "metadata_set:{}:{}",
            metadata_target_name(*target),
            required_keys.join(",")
        ),
        SchemaProjectionSelector::AvailabilityReason { reason_id } => {
            format!("availability_reason:{reason_id}")
        }
        _ => "unknown".to_owned(),
    }
}

pub(super) fn projection_output_target_name(target: &ProjectionOutputTarget) -> &'static str {
    match target {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
        _ => "unknown",
    }
}

pub(super) fn effect_mode_name(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}
