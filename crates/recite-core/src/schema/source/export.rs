mod basic;
mod projection;
mod provenance;

use crate::ProjectSchema;

/// Emit deterministic generated JSON from the canonical schema.
///
/// The exporter is intentionally read-only. The canonical model has no
/// unserialisable values, so a serialization failure is an invariant breach,
/// not a reason to return a misleading empty manifest.
#[allow(clippy::expect_used)]
pub(super) fn export_json(schema: &ProjectSchema) -> String {
    let mut root = serde_json::Map::new();
    root.insert(
        "schema_version".to_owned(),
        serde_json::json!(schema.schema_version),
    );
    if let Some(metadata) = &schema.producer_metadata {
        if let Some(producer) = &metadata.producer {
            root.insert(
                "producer".to_owned(),
                serde_json::json!({ "kind": producer.kind(), "id": producer.id() }),
            );
        }
        provenance::insert_manifest_metadata(&mut root, Some(metadata));
    }
    basic::insert_sections(&mut root, schema);
    projection::insert_sections(&mut root, schema);
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(root))
        .expect("canonical schema JSON export must contain only serializable values");
    format!("{json}\n")
}

pub(super) fn insert_object(
    root: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
    object: serde_json::Map<String, serde_json::Value>,
) {
    root.insert(name.to_owned(), serde_json::Value::Object(object));
}

pub(super) fn json_literal_string(value: &str) -> String {
    if value.starts_with('$') {
        format!("${value}")
    } else {
        value.to_owned()
    }
}
