use super::super::producer::{RawContentFingerprint, RawProducerFingerprint, RawProducerIdentity};
use super::super::spans::ManifestSpans;
use super::super::validate::validate_non_empty_string;
use crate::schema::{
    ProducerContentFingerprintError, ProducerIdentity, ProducerMetadata,
    producer_content_fingerprint_detailed, schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

pub(super) use super::producer_provenance::{
    ProvenanceLocation, lower_origin, lower_origin_map, lower_origin_value_map,
    lower_producer_fingerprints, origin_entries, validate_origin_keys,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_producer_metadata(
    spans: &mut ManifestSpans,
    file: &str,
    source: &str,
    producer: Option<RawProducerIdentity>,
    content_fingerprint: Option<RawContentFingerprint>,
    schema_export_version: Option<u32>,
    inclusion_policy: Option<String>,
    producer_fingerprints: Vec<RawProducerFingerprint>,
    allow_duplicate_fingerprints: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducerMetadata> {
    if schema_export_version == Some(0) {
        diagnostics.push(schema_diagnostic(
            super::super::diagnostics::MALFORMED_SHAPE,
            "diagnostic-schema-001-producer-export-version",
            "schema_export_version must be greater than zero",
            spans.root_key_span(file, source, "schema_export_version"),
            std::iter::empty::<(String, DiagnosticArgumentValue)>(),
        ));
    }
    let producer = producer.and_then(|raw| {
        let kind_valid = validate_non_empty_string(
            diagnostics,
            "manifest producer kind",
            &raw.kind,
            spans.root_object_value_span(file, source, "producer", "kind"),
        );
        let id_valid = validate_non_empty_string(
            diagnostics,
            "manifest producer id",
            &raw.id,
            spans.root_object_value_span(file, source, "producer", "id"),
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
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::MALFORMED_SHAPE,
                    presentation_id,
                    format!("manifest content_fingerprint is invalid: {error}"),
                    spans.root_key_span(file, source, "content_fingerprint"),
                    arguments,
                ));
                None
            }
        }
    });
    if let Some(policy) = &inclusion_policy {
        validate_non_empty_string(
            diagnostics,
            "inclusion_policy",
            policy,
            spans.root_key_span(file, source, "inclusion_policy"),
        );
    }

    let fingerprints = lower_producer_fingerprints(
        spans,
        file,
        source,
        diagnostics,
        producer_fingerprints,
        &[],
        "manifest",
        spans.root_key_span(file, source, "producer_fingerprints"),
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
