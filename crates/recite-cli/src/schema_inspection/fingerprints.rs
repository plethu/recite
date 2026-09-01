use recite_compiler::{ProducerFingerprintScopes, SchemaSummary};
use recite_core::{ContentFingerprint, ProducerFingerprint, SchemaFingerprint};

use crate::error::CliError;

use super::error::SchemaInspectionError;
use super::model::{
    FingerprintProjection, FingerprintScopesProjection, FingerprintsProjection,
    ProducerFingerprintProjection,
};

pub(super) fn fingerprints_json(
    summary: &SchemaSummary,
    schema: &recite_core::ProjectSchema,
) -> Result<FingerprintsProjection, CliError> {
    let scopes = ProducerFingerprintScopes::from_schema(schema).map_err(|error| {
        CliError::SchemaInspection(SchemaInspectionError::InvalidSummary {
            reason: error.to_string(),
        })
    })?;
    Ok(FingerprintsProjection {
        semantic: schema_fingerprint_json(summary.semantic_fingerprint()),
        content: content_fingerprint_json(summary.fingerprints().canonical_content()),
        source: summary
            .fingerprints()
            .source_owned()
            .map(content_fingerprint_json),
        producer_content: summary
            .fingerprints()
            .producer_content()
            .map(content_fingerprint_json),
        producer_inputs: scopes_json(&scopes),
    })
}

pub(super) fn scopes_json(scopes: &ProducerFingerprintScopes) -> FingerprintScopesProjection {
    FingerprintScopesProjection {
        manifest: scopes
            .manifest()
            .iter()
            .map(producer_fingerprint_projection)
            .collect(),
        registries: scopes
            .registries()
            .iter()
            .map(|(name, fingerprints)| {
                (
                    name.clone(),
                    fingerprints
                        .iter()
                        .map(producer_fingerprint_projection)
                        .collect(),
                )
            })
            .collect(),
        metadata_domains: scopes
            .metadata_domains()
            .iter()
            .map(|(name, fingerprints)| {
                (
                    name.clone(),
                    fingerprints
                        .iter()
                        .map(producer_fingerprint_projection)
                        .collect(),
                )
            })
            .collect(),
    }
}

pub(super) fn producer_fingerprint_projection(
    value: &ProducerFingerprint,
) -> ProducerFingerprintProjection {
    ProducerFingerprintProjection {
        kind: value.kind.clone(),
        id: value.id.clone(),
        algorithm: value.algorithm.clone(),
        value: value.value.clone(),
    }
}

pub(super) fn content_fingerprint_json(value: &ContentFingerprint) -> FingerprintProjection {
    FingerprintProjection {
        algorithm: value.algorithm().as_str().to_owned(),
        value: value
            .digest()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    }
}

pub(super) fn schema_fingerprint_json(value: &SchemaFingerprint) -> serde_json::Value {
    match value {
        SchemaFingerprint::Fingerprint(value) => {
            serde_json::json!({ "kind": "fingerprint", "value": content_fingerprint_json(value) })
        }
        SchemaFingerprint::NoSchema => serde_json::json!({ "kind": "no_schema" }),
    }
}
