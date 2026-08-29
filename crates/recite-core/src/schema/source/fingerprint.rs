use crate::{ContentFingerprint, ProducerFingerprint, ProjectSchema};

/// Compute the source-owned fingerprint: canonical semantic content plus the
/// stable producer identity. Formatting, comments, and map insertion order do
/// not affect it; semantic arrays are retained by the canonical model.
pub(super) fn source_fingerprint(schema: &ProjectSchema) -> ContentFingerprint {
    let mut bytes = Vec::from(b"recite-schema-source-fingerprint-v1\0".as_slice());
    if let Some(metadata) = &schema.producer_metadata
        && let Some(producer) = &metadata.producer
    {
        bytes.extend_from_slice(producer.kind.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(producer.id.as_bytes());
    }
    bytes.push(0);
    bytes.extend_from_slice(schema.canonical_content_fingerprint().digest().as_bytes());
    #[allow(clippy::expect_used)]
    {
        ContentFingerprint::blake3(blake3::hash(&bytes).as_bytes().to_vec())
            .expect("BLAKE3 always produces a valid content fingerprint")
    }
}

pub(super) fn source_producer_fingerprint(
    schema: &ProjectSchema,
    source_fingerprint: &ContentFingerprint,
) -> Option<ProducerFingerprint> {
    let producer = schema.producer_metadata.as_ref()?.producer.as_ref()?;
    Some(ProducerFingerprint {
        id: producer.id.clone(),
        kind: producer.kind.clone(),
        algorithm: source_fingerprint.algorithm().as_str().to_owned(),
        value: source_fingerprint
            .digest()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}
