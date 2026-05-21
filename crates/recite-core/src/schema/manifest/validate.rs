use crate::{Diagnostic, EffectMode, SourceSpan};

use crate::schema::{MetadataTarget, ProjectSchema, SchemaTypeRef};

use super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE, diagnostic,
};

#[derive(Clone, Debug)]
pub(crate) struct PendingTypeReference {
    pub(crate) owner: String,
    pub(crate) type_ref: SchemaTypeRef,
    pub(crate) span: SourceSpan,
}

pub(crate) fn validate_type_references(
    schema: &ProjectSchema,
    pending_type_refs: &[PendingTypeReference],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for pending in pending_type_refs {
        validate_type_ref(schema, diagnostics, pending);
    }
}

fn validate_type_ref(
    schema: &ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending: &PendingTypeReference,
) {
    match &pending.type_ref {
        SchemaTypeRef::Enum(name) if !schema.types.contains_key(name) => {
            diagnostics.push(diagnostic(
                INVALID_TYPE_REFERENCE,
                format!("{} references unknown enum type '{name}'", pending.owner),
                pending.span.clone(),
            ));
        }
        SchemaTypeRef::Registry(name) if !schema.registries.contains_key(name) => {
            diagnostics.push(diagnostic(
                INVALID_TYPE_REFERENCE,
                format!("{} references unknown registry '{name}'", pending.owner),
                pending.span.clone(),
            ));
        }
        _ => {}
    }
}

pub(crate) fn duplicate_definition(
    diagnostics: &mut Vec<Diagnostic>,
    kind: &str,
    name: &str,
    span: SourceSpan,
) {
    diagnostics.push(diagnostic(
        DUPLICATE_DEFINITION,
        format!("duplicate {kind} definition '{name}'"),
        span,
    ));
}

pub(crate) fn validate_non_empty_string(
    diagnostics: &mut Vec<Diagnostic>,
    field: &str,
    value: &str,
    span: SourceSpan,
) -> bool {
    if value.is_empty() {
        diagnostics.push(diagnostic(
            MALFORMED_SHAPE,
            format!("{field} must not be empty"),
            span,
        ));
        return false;
    }

    true
}

pub(crate) fn parse_type_ref(value: &str) -> Option<SchemaTypeRef> {
    match value {
        "string" => Some(SchemaTypeRef::String),
        "int" => Some(SchemaTypeRef::Int),
        "float" => Some(SchemaTypeRef::Float),
        "bool" => Some(SchemaTypeRef::Bool),
        "speaker" => Some(SchemaTypeRef::Speaker),
        _ => value
            .strip_prefix("enum:")
            .filter(|name| is_manifest_ref_name(name))
            .map(|name| SchemaTypeRef::Enum(name.to_owned()))
            .or_else(|| {
                value
                    .strip_prefix("registry:")
                    .filter(|name| is_manifest_ref_name(name))
                    .map(|name| SchemaTypeRef::Registry(name.to_owned()))
            }),
    }
}

pub(crate) fn parse_enum_return(value: &str) -> Option<String> {
    value
        .strip_prefix("enum:")
        .filter(|name| is_manifest_ref_name(name))
        .map(ToOwned::to_owned)
}

fn is_manifest_ref_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || character == '.'
                || character == ':'
                || character == '-'
        })
}

pub(crate) fn parse_effect_mode(value: &str) -> Option<EffectMode> {
    match value {
        "deferred" => Some(EffectMode::Deferred),
        "immediate" => Some(EffectMode::Immediate),
        "blocking" => Some(EffectMode::Blocking),
        _ => None,
    }
}

pub(crate) fn parse_metadata_target(value: &str) -> Option<MetadataTarget> {
    match value {
        "block" => Some(MetadataTarget::Block),
        "choice" => Some(MetadataTarget::Choice),
        "line" => Some(MetadataTarget::Line),
        "project" => Some(MetadataTarget::Project),
        _ => None,
    }
}
