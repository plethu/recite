use std::collections::BTreeMap;

use super::super::diagnostics::MALFORMED_SHAPE;
use super::super::producer::{RawProducerFingerprint, RawProducerOrigin};
use super::super::spans::ManifestSpans;
use super::producer::{
    ProvenanceLocation, lower_origin, lower_origin_map, lower_producer_fingerprints, origin_entries,
};
use crate::Diagnostic;
use crate::DiagnosticArgumentValue;
use crate::SourceSpan;
use crate::schema::{ContextualMetadataProvenance, FlatMetadataProvenance, schema_diagnostic};
use serde_json::Value;

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_domain_kind_fields(
    diagnostics: &mut Vec<Diagnostic>,
    kind: &str,
    has_values: bool,
    has_selector: bool,
    has_values_by_context: bool,
    has_missing_context: bool,
    has_context_origins: bool,
    owner: &str,
    span: SourceSpan,
) -> bool {
    let wrong_fields: &[(bool, &str)] = match kind {
        "flat" => &[
            (has_selector, "selector"),
            (has_values_by_context, "values_by_context"),
            (has_missing_context, "missing_context"),
            (has_context_origins, "context_origins"),
        ],
        "contextual" => &[(has_values, "values")],
        _ => &[],
    };
    for &(present, field) in wrong_fields {
        if present {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-domain-kind-field",
                format!("metadata domain '{owner}' does not allow '{field}' for kind '{kind}'"),
                span.clone(),
                [
                    ("domain", DiagnosticArgumentValue::String(owner.to_owned())),
                    ("field", DiagnosticArgumentValue::String(field.to_owned())),
                    ("kind", DiagnosticArgumentValue::String(kind.to_owned())),
                ],
            ));
        }
    }
    !wrong_fields.iter().any(|(present, _)| *present)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_flat_domain_provenance(
    spans: &mut ManifestSpans,
    file: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
    origin: Option<RawProducerOrigin>,
    value_origins: Option<Value>,
    producer_fingerprints: Vec<RawProducerFingerprint>,
    owner: &str,
    span: SourceSpan,
    allow_duplicate_fingerprints: bool,
    parent_path: &[String],
) -> FlatMetadataProvenance {
    let value_origins = match value_origins {
        None => BTreeMap::new(),
        Some(value) => match origin_entries(value) {
            Ok(origins) => {
                let mut value_origins_path = parent_path.to_vec();
                value_origins_path.push("value_origins".to_owned());
                lower_origin_map(
                    spans,
                    file,
                    source,
                    diagnostics,
                    origins,
                    ProvenanceLocation {
                        owner: &format!("{owner} value"),
                        span: span.clone(),
                        path: &value_origins_path,
                    },
                )
            }
            Err(_) => {
                diagnostics.push(schema_diagnostic(
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
                spans,
                file,
                source,
                diagnostics,
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
            spans,
            file,
            source,
            diagnostics,
            producer_fingerprints,
            parent_path,
            owner,
            span,
            allow_duplicate_fingerprints,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_contextual_domain_provenance(
    spans: &mut ManifestSpans,
    file: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
    origin: Option<RawProducerOrigin>,
    context_origins: Option<Value>,
    value_origins: Option<Value>,
    producer_fingerprints: Vec<RawProducerFingerprint>,
    owner: &str,
    span: SourceSpan,
    allow_duplicate_fingerprints: bool,
    parent_path: &[String],
) -> ContextualMetadataProvenance {
    let value_origins = match value_origins {
        None => BTreeMap::new(),
        Some(value) => match object_entries(value) {
            Ok(origins) => origins
                .into_iter()
                .filter_map(|(context, value)| {
                    if !crate::is_valid_source_label(&context) {
                        diagnostics.push(schema_diagnostic(
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
                                .extend(["value_origins".to_owned(), context.clone()]);
                            lower_origin_map(
                                spans,
                                file,
                                source,
                                diagnostics,
                                value_origins,
                                ProvenanceLocation {
                                    owner: &format!("{owner} context value"),
                                    span: span.clone(),
                                    path: &value_origins_path,
                                },
                            )
                        }
                        Err(_) => {
                            diagnostics.push(schema_diagnostic(
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
                    Some((context, value_origins))
                })
                .collect(),
            Err(_) => {
                diagnostics.push(schema_diagnostic(
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
                spans,
                file,
                source,
                diagnostics,
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
                        spans,
                        file,
                        source,
                        diagnostics,
                        origins,
                        ProvenanceLocation {
                            owner: &format!("{owner} context"),
                            span: span.clone(),
                            path: &context_origins_path,
                        },
                    )
                }
                Err(_) => {
                    diagnostics.push(schema_diagnostic(
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
            spans,
            file,
            source,
            diagnostics,
            producer_fingerprints,
            parent_path,
            owner,
            span,
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
