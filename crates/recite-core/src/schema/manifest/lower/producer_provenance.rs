use std::collections::{BTreeMap, BTreeSet};

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::producer::{RawProducerFingerprint, RawProducerOrigin};
use super::super::validate::validate_non_empty_string;
use super::LoweringContext;
use crate::schema::{ProducerFingerprint, ProducerMetadataValue, ProducerOrigin};
use crate::schema::{is_namespaced_extension_key, schema_diagnostic};
use crate::{DiagnosticArgumentValue, SourceSpan};
use serde_json::Value;

pub(super) struct ProvenanceLocation<'a> {
    pub(super) owner: &'a str,
    pub(super) span: SourceSpan,
    pub(super) path: &'a [String],
}

pub(super) fn lower_origin(
    context: &mut LoweringContext<'_>,
    raw: Option<RawProducerOrigin>,
    location: ProvenanceLocation<'_>,
) -> Option<ProducerOrigin> {
    let raw = raw?;

    let mut kind_path = location.path.to_vec();
    kind_path.push("kind".to_owned());
    let mut id_path = location.path.to_vec();
    id_path.push("id".to_owned());

    let kind_span = context.value_span_at(&kind_path, &raw.kind);
    let kind_valid = validate_non_empty_string(
        context.diagnostics,
        &format!("{} origin kind", location.owner),
        &raw.kind,
        kind_span,
    );
    let id_span = context.value_span_at(&id_path, &raw.id);
    let id_valid = validate_non_empty_string(
        context.diagnostics,
        &format!("{} origin id", location.owner),
        &raw.id,
        id_span,
    );
    let label_valid = raw.label.as_ref().is_none_or(|label| {
        let mut label_path = location.path.to_vec();
        label_path.push("label".to_owned());
        let label_span = context.value_span_at(&label_path, label);
        validate_non_empty_string(
            context.diagnostics,
            &format!("{} origin label", location.owner),
            label,
            label_span,
        )
    });

    for key in raw.extensions.keys() {
        if !is_namespaced_extension_key(key) {
            let mut extension_path = location.path.to_vec();
            extension_path.push(key.clone());
            let extension_span = context.nested_key_span_at(&extension_path, key);
            context.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-origin-extension",
                format!(
                    "{} origin extension '{key}' must be namespaced",
                    location.owner
                ),
                extension_span,
                [
                    (
                        "owner",
                        DiagnosticArgumentValue::String(location.owner.to_owned()),
                    ),
                    ("key", DiagnosticArgumentValue::String(key.clone())),
                ],
            ));
        }
    }

    (kind_valid && id_valid && label_valid).then_some(ProducerOrigin {
        kind: raw.kind,
        id: raw.id,
        label: raw.label,
        extensions: raw
            .extensions
            .into_iter()
            .map(|(key, value)| (key, ProducerMetadataValue::from_json(value)))
            .collect(),
    })
}

pub(super) fn lower_origin_map(
    context: &mut LoweringContext<'_>,
    raw: impl IntoIterator<Item = (String, RawProducerOrigin)>,
    location: ProvenanceLocation<'_>,
) -> BTreeMap<String, ProducerOrigin> {
    raw.into_iter()
        .filter_map(|(key, origin)| {
            let mut origin_path = location.path.to_vec();
            origin_path.push(key.clone());
            let key_span = context.nested_key_span_at(&origin_path, &key);
            if !validate_non_empty_string(
                context.diagnostics,
                &format!("{} provenance key", location.owner),
                &key,
                key_span,
            ) {
                return None;
            }
            lower_origin(
                context,
                Some(origin),
                ProvenanceLocation {
                    owner: &format!("{} '{key}'", location.owner),
                    span: location.span.clone(),
                    path: &origin_path,
                },
            )
            .map(|origin| (key, origin))
        })
        .collect()
}

pub(super) fn lower_origin_value_map(
    context: &mut LoweringContext<'_>,
    raw: Option<Value>,
    location: ProvenanceLocation<'_>,
) -> BTreeMap<String, ProducerOrigin> {
    let Some(value) = raw else {
        return BTreeMap::new();
    };
    let origins = match origin_entries(value) {
        Ok(origins) => origins,
        Err(_) => {
            context.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-value-origins",
                format!(
                    "{} value_origins must map values to origins",
                    location.owner
                ),
                location.span.clone(),
                [(
                    "owner",
                    DiagnosticArgumentValue::String(location.owner.to_owned()),
                )],
            ));
            return BTreeMap::new();
        }
    };
    let mut value_origins_path = location.path.to_vec();
    value_origins_path.push("value_origins".to_owned());
    lower_origin_map(
        context,
        origins,
        ProvenanceLocation {
            owner: location.owner,
            span: location.span,
            path: &value_origins_path,
        },
    )
}

pub(super) fn lower_producer_fingerprints(
    context: &mut LoweringContext<'_>,
    raw: Vec<RawProducerFingerprint>,
    parent_path: &[String],
    owner: &str,
    allow_duplicate_fingerprints: bool,
) -> Vec<ProducerFingerprint> {
    let mut seen = BTreeSet::new();
    let mut lowered = raw
        .into_iter()
        .enumerate()
        .filter_map(|(index, fingerprint)| {
            let mut fingerprint_path = parent_path.to_vec();
            fingerprint_path.extend(["producer_fingerprints".to_owned(), format!("[{index}]")]);
            let mut id_path = fingerprint_path.clone();
            id_path.push("id".to_owned());
            let id_span = context.value_span_at(&id_path, &fingerprint.id);
            let id_valid = validate_non_empty_string(
                context.diagnostics,
                &format!("{owner} producer fingerprint id"),
                &fingerprint.id,
                id_span.clone(),
            );
            let kind_span = context.value_span_at(
                &fingerprint_field_path(&fingerprint_path, "kind"),
                &fingerprint.kind,
            );
            let kind_valid = validate_non_empty_string(
                context.diagnostics,
                &format!("{owner} producer fingerprint kind"),
                &fingerprint.kind,
                kind_span,
            );
            let algorithm_span = context.value_span_at(
                &fingerprint_field_path(&fingerprint_path, "algorithm"),
                &fingerprint.algorithm,
            );
            let algorithm_valid = validate_non_empty_string(
                context.diagnostics,
                &format!("{owner} producer fingerprint algorithm"),
                &fingerprint.algorithm,
                algorithm_span,
            );
            let value_span = context.value_span_at(
                &fingerprint_field_path(&fingerprint_path, "value"),
                &fingerprint.value,
            );
            let value_valid = validate_non_empty_string(
                context.diagnostics,
                &format!("{owner} producer fingerprint value"),
                &fingerprint.value,
                value_span,
            );
            if !(id_valid && kind_valid && algorithm_valid && value_valid) {
                return None;
            }

            if !seen.insert((fingerprint.kind.clone(), fingerprint.id.clone()))
                && !allow_duplicate_fingerprints
            {
                context.diagnostics.push(schema_diagnostic(
                    DUPLICATE_DEFINITION,
                    "diagnostic-schema-003-producer-fingerprint",
                    format!(
                        "{owner} repeats producer fingerprint '{}:{}'",
                        fingerprint.kind, fingerprint.id
                    ),
                    id_span,
                    [
                        ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
                        (
                            "kind",
                            DiagnosticArgumentValue::String(fingerprint.kind.clone()),
                        ),
                        (
                            "id",
                            DiagnosticArgumentValue::String(fingerprint.id.clone()),
                        ),
                    ],
                ));
                return None;
            }

            Some(ProducerFingerprint {
                id: fingerprint.id,
                kind: fingerprint.kind,
                algorithm: fingerprint.algorithm,
                value: fingerprint.value,
            })
        })
        .collect::<Vec<_>>();
    lowered.sort();
    lowered
}

pub(super) fn origin_entries(value: Value) -> Result<Vec<(String, RawProducerOrigin)>, String> {
    let Value::Object(entries) = value else {
        return Err("producer origins must be an object".to_owned());
    };
    entries
        .into_iter()
        .map(|(name, value)| {
            serde_json::from_value(value)
                .map(|origin| (name, origin))
                .map_err(|error| error.to_string())
        })
        .collect()
}

pub(super) fn validate_origin_keys(
    context: &mut LoweringContext<'_>,
    owner: &str,
    allowed: &BTreeSet<String>,
    origins: impl IntoIterator<Item = String>,
    path: &[String],
) {
    for key in origins {
        if !allowed.contains(&key) {
            let mut origin_path = path.to_vec();
            origin_path.push(key.clone());
            let unknown_span = context.nested_key_span_at(&origin_path, &key);
            context.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-provenance-unknown-value",
                format!("{owner} provenance key '{key}' is not a declared value"),
                unknown_span,
                [
                    ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
                    ("key", DiagnosticArgumentValue::String(key.clone())),
                ],
            ));
        }
    }
}

fn fingerprint_field_path(parent: &[String], field: &str) -> Vec<String> {
    let mut path = parent.to_vec();
    path.push(field.to_owned());
    path
}
