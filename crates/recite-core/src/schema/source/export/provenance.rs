use std::collections::BTreeMap;
use std::str::FromStr;

use crate::schema::{
    ContextualMetadataProvenance, FlatMetadataProvenance, ProducerFingerprint, ProducerMetadata,
    ProducerMetadataValue, ProducerOrigin,
};

pub(super) fn insert_manifest_metadata(
    root: &mut serde_json::Map<String, serde_json::Value>,
    metadata: Option<&ProducerMetadata>,
) {
    let Some(metadata) = metadata else {
        return;
    };
    if let Some(content) = &metadata.content_fingerprint {
        root.insert(
            "content_fingerprint".to_owned(),
            serde_json::json!({
                "algorithm": content.algorithm().as_str(),
                "value": content.digest().as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>()
            }),
        );
    }
    if let Some(version) = metadata.schema_export_version {
        root.insert(
            "schema_export_version".to_owned(),
            serde_json::json!(version),
        );
    }
    if let Some(policy) = &metadata.inclusion_policy {
        root.insert("inclusion_policy".to_owned(), serde_json::json!(policy));
    }
    if !metadata.producer_fingerprints.is_empty() {
        root.insert(
            "producer_fingerprints".to_owned(),
            json_fingerprints(&metadata.producer_fingerprints),
        );
    }
}

pub(super) fn add_flat_provenance(
    value: &mut serde_json::Map<String, serde_json::Value>,
    provenance: &FlatMetadataProvenance,
) {
    add_origin(value, provenance.origin.as_ref());
    add_origin_map(value, "value_origins", &provenance.value_origins);
    add_fingerprints(value, &provenance.producer_fingerprints);
}

pub(super) fn add_contextual_provenance(
    value: &mut serde_json::Map<String, serde_json::Value>,
    provenance: &ContextualMetadataProvenance,
) {
    add_origin(value, provenance.origin.as_ref());
    add_origin_map(value, "context_origins", &provenance.context_origins);
    let values = provenance
        .value_origins
        .iter()
        .map(|(context, origins)| (context.clone(), json_origin_map(origins)))
        .collect::<serde_json::Map<_, _>>();
    if !values.is_empty() {
        value.insert(
            "value_origins".to_owned(),
            serde_json::Value::Object(values),
        );
    }
    add_fingerprints(value, &provenance.producer_fingerprints);
}

pub(super) fn add_origin(
    value: &mut serde_json::Map<String, serde_json::Value>,
    origin: Option<&ProducerOrigin>,
) {
    if let Some(origin) = origin {
        value.insert("origin".to_owned(), json_origin(origin));
    }
}

fn add_origin_map(
    value: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
    origins: &BTreeMap<String, ProducerOrigin>,
) {
    if !origins.is_empty() {
        value.insert(name.to_owned(), json_origin_map(origins));
    }
}

fn add_fingerprints(
    value: &mut serde_json::Map<String, serde_json::Value>,
    fingerprints: &[ProducerFingerprint],
) {
    if !fingerprints.is_empty() {
        value.insert(
            "producer_fingerprints".to_owned(),
            json_fingerprints(fingerprints),
        );
    }
}

pub(super) fn json_origin_map(origins: &BTreeMap<String, ProducerOrigin>) -> serde_json::Value {
    serde_json::Value::Object(
        origins
            .iter()
            .map(|(key, origin)| (key.clone(), json_origin(origin)))
            .collect(),
    )
}

fn json_origin(origin: &ProducerOrigin) -> serde_json::Value {
    let mut value = serde_json::Map::new();
    value.insert("kind".to_owned(), serde_json::json!(origin.kind));
    value.insert("id".to_owned(), serde_json::json!(origin.id));
    if let Some(label) = &origin.label {
        value.insert("label".to_owned(), serde_json::json!(label));
    }
    for (key, extension) in &origin.extensions {
        value.insert(key.clone(), json_metadata_value(extension));
    }
    serde_json::Value::Object(value)
}

pub(super) fn json_fingerprints(fingerprints: &[ProducerFingerprint]) -> serde_json::Value {
    serde_json::Value::Array(
        fingerprints
            .iter()
            .map(|fingerprint| {
                serde_json::json!({
                    "id": fingerprint.id,
                    "kind": fingerprint.kind,
                    "algorithm": fingerprint.algorithm,
                    "value": fingerprint.value,
                })
            })
            .collect(),
    )
}

fn json_metadata_value(value: &ProducerMetadataValue) -> serde_json::Value {
    match value {
        ProducerMetadataValue::Null => serde_json::Value::Null,
        ProducerMetadataValue::Bool(value) => serde_json::json!(value),
        ProducerMetadataValue::Number(value) => serde_json::Number::from_str(value)
            .map_or_else(|_| serde_json::json!(value), serde_json::Value::Number),
        ProducerMetadataValue::String(value) => serde_json::json!(value),
        ProducerMetadataValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(json_metadata_value).collect())
        }
        ProducerMetadataValue::Object(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), json_metadata_value(value)))
                .collect(),
        ),
    }
}
