use crate::{Diagnostic, EffectMode, SourceSpan};

use crate::schema::{
    MetadataContextSelector, MetadataDomainDefinition, MetadataTarget, ProjectSchema, SchemaTypeRef,
};

use super::diagnostics::{DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};

#[derive(Clone, Debug)]
pub(crate) struct PendingTypeReference {
    pub(crate) owner: String,
    pub(crate) type_ref: SchemaTypeRef,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingDomainReference {
    pub(crate) owner: String,
    pub(crate) domain: String,
    pub(crate) require_flat: bool,
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

pub(crate) fn validate_domain_references(
    schema: &ProjectSchema,
    pending_domain_refs: &[PendingDomainReference],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for pending in pending_domain_refs {
        match schema.metadata_domains.get(&pending.domain) {
            Some(MetadataDomainDefinition::Flat(_)) => {}
            Some(MetadataDomainDefinition::Contextual(_)) if pending.require_flat => {
                diagnostics.push(Diagnostic::error(
                    INVALID_TYPE_REFERENCE,
                    format!(
                        "{} references contextual metadata domain '{}', but a flat domain is required",
                        pending.owner, pending.domain
                    ),
                    pending.span.clone(),
                ));
            }
            Some(MetadataDomainDefinition::Contextual(_)) => {}
            None => diagnostics.push(Diagnostic::error(
                INVALID_TYPE_REFERENCE,
                format!(
                    "{} references unknown metadata domain '{}'",
                    pending.owner, pending.domain
                ),
                pending.span.clone(),
            )),
        }
    }
}

fn validate_type_ref(
    schema: &ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending: &PendingTypeReference,
) {
    match &pending.type_ref {
        SchemaTypeRef::Enum(name) if !schema.types.contains_key(name) => {
            diagnostics.push(Diagnostic::error(
                INVALID_TYPE_REFERENCE,
                format!("{} references unknown enum type '{name}'", pending.owner),
                pending.span.clone(),
            ));
        }
        SchemaTypeRef::Registry(name) if !schema.registries.contains_key(name) => {
            diagnostics.push(Diagnostic::error(
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
    diagnostics.push(Diagnostic::error(
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
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("{field} must not be empty"),
            span,
        ));
        return false;
    }

    true
}

pub(crate) fn validate_manifest_name(
    diagnostics: &mut Vec<Diagnostic>,
    field: &str,
    value: &str,
    span: SourceSpan,
) -> bool {
    if is_manifest_name(value) {
        return true;
    }

    diagnostics.push(Diagnostic::error(
        MALFORMED_SHAPE,
        format!("{field} must be an identifier-like schema name"),
        span,
    ));
    false
}

pub(crate) fn parse_type_ref(value: &str) -> Option<SchemaTypeRef> {
    match value {
        "string" => Some(SchemaTypeRef::String),
        "symbol" => Some(SchemaTypeRef::Symbol),
        "int" => Some(SchemaTypeRef::Int),
        "float" => Some(SchemaTypeRef::Float),
        "bool" => Some(SchemaTypeRef::Bool),
        "speaker" => Some(SchemaTypeRef::Speaker),
        _ => value
            .strip_prefix("enum:")
            .filter(|name| is_manifest_name(name))
            .map(|name| SchemaTypeRef::Enum(name.to_owned()))
            .or_else(|| {
                value
                    .strip_prefix("registry:")
                    .filter(|name| is_manifest_name(name))
                    .map(|name| SchemaTypeRef::Registry(name.to_owned()))
            }),
    }
}

pub(crate) fn parse_metadata_context_selector(value: &str) -> Option<MetadataContextSelector> {
    match value {
        "field:speaker" => Some(MetadataContextSelector::FieldSpeaker),
        _ => value
            .strip_prefix("metadata:")
            .filter(|name| is_manifest_name(name))
            .map(|name| MetadataContextSelector::MetadataKey(name.to_owned())),
    }
}

pub(crate) fn parse_enum_return(value: &str) -> Option<String> {
    value
        .strip_prefix("enum:")
        .filter(|name| is_manifest_name(name))
        .map(ToOwned::to_owned)
}

fn is_manifest_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || character == '.'
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
