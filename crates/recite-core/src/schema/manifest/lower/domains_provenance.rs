use std::collections::BTreeMap;

use super::super::diagnostics::MALFORMED_SHAPE;
use super::super::producer::{RawProducerFingerprint, RawProducerOrigin};
use super::LoweringContext;
use super::producer::{
    ProvenanceLocation, lower_origin, lower_origin_map, lower_producer_fingerprints, origin_entries,
};
use crate::Diagnostic;
use crate::DiagnosticArgumentValue;
use crate::SourceSpan;
use crate::schema::{ContextualMetadataProvenance, FlatMetadataProvenance, schema_diagnostic};
use serde_json::Value;

pub(super) struct DomainKindFields<'a> {
    pub(super) kind: &'a str,
    pub(super) has_values: bool,
    pub(super) has_selector: bool,
    pub(super) has_values_by_context: bool,
    pub(super) has_missing_context: bool,
    pub(super) has_context_origins: bool,
    pub(super) owner: &'a str,
    pub(super) span: SourceSpan,
}

pub(super) struct FlatDomainProvenanceInput<'a> {
    pub(super) origin: Option<RawProducerOrigin>,
    pub(super) value_origins: Option<Value>,
    pub(super) producer_fingerprints: Vec<RawProducerFingerprint>,
    pub(super) location: ProvenanceLocation<'a>,
    pub(super) allow_duplicate_fingerprints: bool,
}

pub(super) struct ContextualDomainProvenanceInput<'a> {
    pub(super) origin: Option<RawProducerOrigin>,
    pub(super) context_origins: Option<Value>,
    pub(super) value_origins: Option<Value>,
    pub(super) producer_fingerprints: Vec<RawProducerFingerprint>,
    pub(super) location: ProvenanceLocation<'a>,
    pub(super) allow_duplicate_fingerprints: bool,
}

pub(super) fn validate_domain_kind_fields(
    diagnostics: &mut Vec<Diagnostic>,
    fields: DomainKindFields<'_>,
) -> bool {
    let wrong_fields: &[(bool, &str)] = match fields.kind {
        "flat" => &[
            (fields.has_selector, "selector"),
            (fields.has_values_by_context, "values_by_context"),
            (fields.has_missing_context, "missing_context"),
            (fields.has_context_origins, "context_origins"),
        ],
        "contextual" => &[(fields.has_values, "values")],
        _ => &[],
    };
    for &(present, field) in wrong_fields {
        if present {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-domain-kind-field",
                format!(
                    "metadata domain '{}' does not allow '{field}' for kind '{}'",
                    fields.owner, fields.kind
                ),
                fields.span.clone(),
                [
                    (
                        "domain",
                        DiagnosticArgumentValue::String(fields.owner.to_owned()),
                    ),
                    ("field", DiagnosticArgumentValue::String(field.to_owned())),
                    (
                        "kind",
                        DiagnosticArgumentValue::String(fields.kind.to_owned()),
                    ),
                ],
            ));
        }
    }
    !wrong_fields.iter().any(|(present, _)| *present)
}

pub(super) fn lower_flat_domain_provenance(
    context: &mut LoweringContext<'_>,
    input: FlatDomainProvenanceInput<'_>,
) -> FlatMetadataProvenance {
    let FlatDomainProvenanceInput {
        origin,
        value_origins,
        producer_fingerprints,
        location,
        allow_duplicate_fingerprints,
    } = input;
    let owner = location.owner;
    let span = location.span;
    let parent_path = location.path;
    let value_origins = match value_origins {
        None => BTreeMap::new(),
        Some(value) => match origin_entries(value) {
            Ok(origins) => {
                let mut value_origins_path = parent_path.to_vec();
                value_origins_path.push("value_origins".to_owned());
                lower_origin_map(
                    context,
                    origins,
                    ProvenanceLocation {
                        owner: &format!("{owner} value"),
                        span: span.clone(),
                        path: &value_origins_path,
                    },
                )
            }
            Err(_) => {
                context.diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-flat-value-origins",
                    format!("{owner} flat value_origins must map values to origins"),
                    span.clone(),
                    [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                ));
                BTreeMap::new()
            }
        },
    };

    FlatMetadataProvenance {
        origin: {
            let mut origin_path = parent_path.to_vec();
            origin_path.push("origin".to_owned());
            lower_origin(
                context,
                origin,
                ProvenanceLocation {
                    owner,
                    span: span.clone(),
                    path: &origin_path,
                },
            )
        },
        value_origins,
        producer_fingerprints: lower_producer_fingerprints(
            context,
            producer_fingerprints,
            parent_path,
            owner,
            allow_duplicate_fingerprints,
        ),
    }
}

pub(super) fn lower_contextual_domain_provenance(
    context: &mut LoweringContext<'_>,
    input: ContextualDomainProvenanceInput<'_>,
) -> ContextualMetadataProvenance {
    let ContextualDomainProvenanceInput {
        origin,
        context_origins,
        value_origins,
        producer_fingerprints,
        location,
        allow_duplicate_fingerprints,
    } = input;
    let owner = location.owner;
    let span = location.span;
    let parent_path = location.path;
    let value_origins = match value_origins {
        None => BTreeMap::new(),
        Some(value) => match object_entries(value) {
            Ok(origins) => origins
                .into_iter()
                .filter_map(|(context_name, value)| {
                    if !crate::is_valid_source_label(&context_name) {
                        context.diagnostics.push(schema_diagnostic(
                            MALFORMED_SHAPE,
                            "diagnostic-schema-001-context-origin-name",
                            format!("{owner} provenance context must be an identifier-like name"),
                            span.clone(),
                            [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                        ));
                        return None;
                    }
                    let value_origins = match origin_entries(value) {
                        Ok(value_origins) => {
                            let mut value_origins_path = parent_path.to_vec();
                            value_origins_path
                                .extend(["value_origins".to_owned(), context_name.clone()]);
                            lower_origin_map(
                                context,
                                value_origins,
                                ProvenanceLocation {
                                    owner: &format!("{owner} context value"),
                                    span: span.clone(),
                                    path: &value_origins_path,
                                },
                            )
                        }
                        Err(_) => {
                            context.diagnostics.push(schema_diagnostic(
                                MALFORMED_SHAPE,
                                "diagnostic-schema-001-contextual-value-origins",
                                format!(
                                    "{owner} contextual value_origins must map contexts to values"
                                ),
                                span.clone(),
                                [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                            ));
                            BTreeMap::new()
                        }
                    };
                    Some((context_name, value_origins))
                })
                .collect(),
            Err(_) => {
                context.diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-contextual-value-origins",
                    format!("{owner} contextual value_origins must map contexts to values"),
                    span.clone(),
                    [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                ));
                BTreeMap::new()
            }
        },
    };

    ContextualMetadataProvenance {
        origin: {
            let mut origin_path = parent_path.to_vec();
            origin_path.push("origin".to_owned());
            lower_origin(
                context,
                origin,
                ProvenanceLocation {
                    owner,
                    span: span.clone(),
                    path: &origin_path,
                },
            )
        },
        context_origins: match context_origins {
            None => BTreeMap::new(),
            Some(value) => match origin_entries(value) {
                Ok(origins) => {
                    let mut context_origins_path = parent_path.to_vec();
                    context_origins_path.push("context_origins".to_owned());
                    lower_origin_map(
                        context,
                        origins,
                        ProvenanceLocation {
                            owner: &format!("{owner} context"),
                            span: span.clone(),
                            path: &context_origins_path,
                        },
                    )
                }
                Err(_) => {
                    context.diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-context-origins",
                        format!("{owner} context_origins must map contexts to origins"),
                        span.clone(),
                        [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                    ));
                    BTreeMap::new()
                }
            },
        },
        value_origins,
        producer_fingerprints: lower_producer_fingerprints(
            context,
            producer_fingerprints,
            parent_path,
            owner,
            allow_duplicate_fingerprints,
        ),
    }
}

fn object_entries(value: Value) -> Result<Vec<(String, Value)>, String> {
    let Value::Object(entries) = value else {
        return Err("producer origins must be an object".to_owned());
    };
    Ok(entries.into_iter().collect())
}
