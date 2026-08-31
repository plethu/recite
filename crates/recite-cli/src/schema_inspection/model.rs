use std::collections::BTreeMap;

use serde::Serialize;

use super::path::MachinePathProjection;

/// Stable CLI projection.  The nested definition values are JSON because the
/// canonical model owns schema semantics; this type owns only their output
/// names, ordering, and provenance envelope.
#[derive(Debug, Serialize)]
pub(super) struct SchemaInspectionProjection {
    pub(super) format_version: u32,
    pub(super) schema_version: u32,
    pub(super) source: SourceProjection,
    pub(super) ownership: OwnershipProjection,
    pub(super) capability: CapabilityProjection,
    pub(super) producer: Option<IdentityProjection>,
    pub(super) fingerprints: FingerprintsProjection,
    pub(super) freshness: FreshnessProjection,
    pub(super) types: Vec<DeclarationProjection>,
    pub(super) registries: Vec<DeclarationProjection>,
    pub(super) speakers: Vec<DeclarationProjection>,
    pub(super) conditions: Vec<DeclarationProjection>,
    pub(super) availability_reasons: Vec<DeclarationProjection>,
    pub(super) effects: Vec<DeclarationProjection>,
    pub(super) metadata_domains: Vec<DeclarationProjection>,
    pub(super) metadata: Vec<DeclarationProjection>,
    pub(super) projection_queries: Vec<DeclarationProjection>,
    pub(super) presentation_projectors: Vec<DeclarationProjection>,
    pub(super) markup: Vec<DeclarationProjection>,
}

#[derive(Debug, Serialize)]
pub(super) struct SourceProjection {
    pub(super) format: &'static str,
    pub(super) path: MachinePathProjection,
    pub(super) read_only: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum OwnershipProjection {
    Standalone { producer: IdentityProjection },
    Generated { producer: IdentityProjection },
    Unavailable,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct IdentityProjection {
    pub(super) kind: String,
    pub(super) id: String,
}

#[derive(Debug, Serialize)]
pub(super) struct FingerprintsProjection {
    pub(super) semantic: serde_json::Value,
    pub(super) content: FingerprintProjection,
    pub(super) source: Option<FingerprintProjection>,
    pub(super) producer_content: Option<FingerprintProjection>,
    pub(super) producer_inputs: FingerprintScopesProjection,
}

#[derive(Debug, Serialize)]
pub(super) struct FingerprintProjection {
    pub(super) algorithm: String,
    pub(super) value: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ProducerFingerprintProjection {
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) algorithm: String,
    pub(super) value: String,
}

#[derive(Debug, Serialize)]
pub(super) struct FingerprintScopesProjection {
    pub(super) manifest: Vec<ProducerFingerprintProjection>,
    pub(super) registries: BTreeMap<String, Vec<ProducerFingerprintProjection>>,
    pub(super) metadata_domains: BTreeMap<String, Vec<ProducerFingerprintProjection>>,
}

#[derive(Debug, Serialize)]
pub(super) struct FreshnessProjection {
    pub(super) status: String,
    pub(super) reason: Option<String>,
    pub(super) channels: Option<FreshnessChannelsProjection>,
}

#[derive(Debug, Serialize)]
pub(super) struct FreshnessChannelsProjection {
    pub(super) content: serde_json::Value,
    pub(super) manifest: serde_json::Value,
    pub(super) registries: BTreeMap<String, serde_json::Value>,
    pub(super) metadata_domains: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct DeclarationProjection {
    pub(super) kind: &'static str,
    pub(super) name: String,
    pub(super) definition: serde_json::Value,
    pub(super) provenance: ProvenanceProjection,
    pub(super) capability: CapabilityProjection,
}

#[derive(Debug, Serialize)]
pub(super) struct ProvenanceProjection {
    pub(super) ownership: OwnershipProjection,
    pub(super) origin: Option<OriginProjection>,
}

#[derive(Debug, Serialize)]
pub(super) struct OriginProjection {
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) label: Option<String>,
    pub(super) extensions: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct CapabilityProjection {
    pub(super) actions: Vec<String>,
    pub(super) unavailable_reasons: Vec<String>,
    pub(super) producer_actions: Vec<ProducerActionProjection>,
}

#[derive(Debug, Serialize)]
pub(super) struct ProducerActionProjection {
    pub(super) request_id: FingerprintProjection,
    pub(super) producer: IdentityProjection,
    pub(super) operation: ProducerOperationProjection,
    pub(super) expected: ProducerEvidenceProjection,
    pub(super) launch: ProducerLaunchProjection,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ProducerOperationProjection {
    Regenerate,
    Retry {
        failure: ProducerFailureProjection,
        originating_request_id: FingerprintProjection,
    },
    Unknown,
}

#[derive(Debug, Serialize)]
pub(super) struct ProducerFailureProjection {
    pub(super) producer: IdentityProjection,
    pub(super) code: String,
    pub(super) detail: Option<String>,
    pub(super) retry_guidance: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ProducerEvidenceProjection {
    pub(super) schema_fingerprint: serde_json::Value,
    pub(super) content_fingerprint: FingerprintProjection,
    pub(super) input_fingerprints: FingerprintScopesProjection,
    pub(super) output_fingerprint: Option<FingerprintProjection>,
}

#[derive(Debug, Serialize)]
pub(super) struct ProducerLaunchProjection {
    pub(super) producer: IdentityProjection,
    pub(super) input_fingerprints: FingerprintScopesProjection,
}
