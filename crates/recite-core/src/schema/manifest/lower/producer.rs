use super::super::producer::{RawContentFingerprint, RawProducerFingerprint, RawProducerIdentity};
use super::super::validate::validate_non_empty_string;
use super::LoweringContext;
use crate::DiagnosticArgumentValue;
use crate::schema::{
    ProducerContentFingerprintError, ProducerIdentity, ProducerMetadata,
    producer_content_fingerprint_detailed, schema_diagnostic,
};

pub(super) use super::producer_provenance::{
    ProvenanceLocation, lower_origin, lower_origin_map, lower_origin_value_map,
    lower_producer_fingerprints, origin_entries, validate_origin_keys,
};

pub(super) struct ProducerMetadataInput {
    pub(super) producer: Option<RawProducerIdentity>,
    pub(super) content_fingerprint: Option<RawContentFingerprint>,
    pub(super) schema_export_version: Option<u32>,
    pub(super) inclusion_policy: Option<String>,
    pub(super) producer_fingerprints: Vec<RawProducerFingerprint>,
    pub(super) allow_duplicate_fingerprints: bool,
}

pub(super) fn lower_producer_metadata(
    context: &mut LoweringContext<'_>,
    input: ProducerMetadataInput,
) -> Option<ProducerMetadata> {
    let ProducerMetadataInput {
        producer,
        content_fingerprint,
        schema_export_version,
        inclusion_policy,
        producer_fingerprints,
        allow_duplicate_fingerprints,
    } = input;
    if schema_export_version == Some(0) {
        context.diagnostics.push(schema_diagnostic(
            super::super::diagnostics::MALFORMED_SHAPE,
            "diagnostic-schema-001-producer-export-version",
            "schema_export_version must be greater than zero",
            context.root_key_span("schema_export_version"),
            std::iter::empty::<(String, DiagnosticArgumentValue)>(),
        ));
    }
    let producer = producer.and_then(|raw| {
        let kind_valid = validate_non_empty_string(
            context.diagnostics,
            "manifest producer kind",
            &raw.kind,
            context.root_object_value_span("producer", "kind"),
        );
        let id_valid = validate_non_empty_string(
            context.diagnostics,
            "manifest producer id",
            &raw.id,
            context.root_object_value_span("producer", "id"),
        );
        if !(kind_valid && id_valid) {
            return None;
        }
        match ProducerIdentity::new(raw.kind, raw.id) {
            Ok(identity) => Some(identity),
            Err(_) => {
                unreachable!("producer identity validation must match manifest field validation")
            }
        }
    });
    let content_fingerprint = content_fingerprint.and_then(|raw| {
        match producer_content_fingerprint_detailed(raw.algorithm, &raw.value) {
            Ok(fingerprint) => Some(fingerprint),
            Err(error) => {
                let (presentation_id, arguments) = match &error {
                    ProducerContentFingerprintError::EmptyAlgorithm => (
                        "diagnostic-schema-001-producer-content-fingerprint-empty-algorithm",
                        Vec::new(),
                    ),
                    ProducerContentFingerprintError::Blake3HexShape => (
                        "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape",
                        Vec::new(),
                    ),
                    ProducerContentFingerprintError::Blake3HexData => (
                        "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data",
                        Vec::new(),
                    ),
                    ProducerContentFingerprintError::EmptyDigest => (
                        "diagnostic-schema-001-producer-content-fingerprint-empty-digest",
                        Vec::new(),
                    ),
                    ProducerContentFingerprintError::Blake3DigestLength { actual } => (
                        "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length",
                        vec![("actual", DiagnosticArgumentValue::Integer(*actual as i64))],
                    ),
                };
                context.diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::MALFORMED_SHAPE,
                    presentation_id,
                    format!("manifest content_fingerprint is invalid: {error}"),
                    context.root_key_span("content_fingerprint"),
                    arguments,
                ));
                None
            }
        }
    });
    if let Some(policy) = &inclusion_policy {
        validate_non_empty_string(
            context.diagnostics,
            "inclusion_policy",
            policy,
            context.root_key_span("inclusion_policy"),
        );
    }

    let fingerprints = lower_producer_fingerprints(
        context,
        producer_fingerprints,
        &[],
        "manifest",
        allow_duplicate_fingerprints,
    );
    if producer.is_none()
        && content_fingerprint.is_none()
        && schema_export_version.is_none()
        && inclusion_policy.is_none()
        && fingerprints.is_empty()
    {
        return None;
    }

    Some(ProducerMetadata {
        producer,
        content_fingerprint,
        schema_export_version,
        inclusion_policy,
        producer_fingerprints: fingerprints,
    })
}
