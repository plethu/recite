use std::collections::BTreeMap;

use recite_core::{MetadataDomainDefinition, ProducerFingerprint, ProducerIdentity, ProjectSchema};

/// Producer fingerprints partitioned by the scope in which they were observed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerFingerprintScopes {
    manifest: Vec<ProducerFingerprint>,
    registries: BTreeMap<String, Vec<ProducerFingerprint>>,
    metadata_domains: BTreeMap<String, Vec<ProducerFingerprint>>,
}

impl ProducerFingerprintScopes {
    #[must_use]
    pub fn new(
        manifest: impl IntoIterator<Item = ProducerFingerprint>,
        registries: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
        metadata_domains: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
    ) -> Self {
        Self {
            manifest: sorted(manifest),
            registries: sorted_map(registries),
            metadata_domains: sorted_map(metadata_domains),
        }
    }

    #[must_use]
    pub fn from_schema(schema: &ProjectSchema) -> Self {
        let manifest = schema
            .producer_metadata
            .as_ref()
            .map_or_else(Vec::new, |metadata| metadata.producer_fingerprints.clone());
        let registries = schema
            .registries
            .iter()
            .map(|(name, registry)| (name.clone(), registry.producer_fingerprints.clone()));
        let metadata_domains = schema.metadata_domains.iter().map(|(name, domain)| {
            let fingerprints = match domain {
                MetadataDomainDefinition::Flat(domain) => {
                    domain.provenance.producer_fingerprints.clone()
                }
                MetadataDomainDefinition::Contextual(domain) => {
                    domain.provenance.producer_fingerprints.clone()
                }
            };
            (name.clone(), fingerprints)
        });
        Self::new(manifest, registries, metadata_domains)
    }

    #[must_use]
    pub fn manifest(&self) -> &[ProducerFingerprint] {
        &self.manifest
    }

    #[must_use]
    pub const fn registries(&self) -> &BTreeMap<String, Vec<ProducerFingerprint>> {
        &self.registries
    }

    #[must_use]
    pub const fn metadata_domains(&self) -> &BTreeMap<String, Vec<ProducerFingerprint>> {
        &self.metadata_domains
    }
}

/// Caller-owned launch/preflight evidence bound into a producer action request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerLaunchSnapshot {
    producer: ProducerIdentity,
    input_fingerprints: ProducerFingerprintScopes,
}

impl ProducerLaunchSnapshot {
    #[must_use]
    pub const fn new(
        producer: ProducerIdentity,
        input_fingerprints: ProducerFingerprintScopes,
    ) -> Self {
        Self {
            producer,
            input_fingerprints,
        }
    }

    pub fn from_schema(schema: &ProjectSchema) -> Result<Self, ProducerLaunchSnapshotError> {
        let producer = schema
            .producer_metadata
            .as_ref()
            .ok_or(ProducerLaunchSnapshotError::MissingProducerMetadata)?
            .producer
            .clone()
            .ok_or(ProducerLaunchSnapshotError::MissingProducerIdentity)?;
        Ok(Self::new(
            producer,
            ProducerFingerprintScopes::from_schema(schema),
        ))
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub const fn input_fingerprints(&self) -> &ProducerFingerprintScopes {
        &self.input_fingerprints
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerLaunchSnapshotError {
    #[error("schema has no producer metadata")]
    MissingProducerMetadata,
    #[error("producer metadata has no producer identity")]
    MissingProducerIdentity,
}

fn sorted(items: impl IntoIterator<Item = ProducerFingerprint>) -> Vec<ProducerFingerprint> {
    let mut items = items.into_iter().collect::<Vec<_>>();
    items.sort();
    items
}

fn sorted_map(
    items: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
) -> BTreeMap<String, Vec<ProducerFingerprint>> {
    items
        .into_iter()
        .map(|(name, fingerprints)| (name, sorted(fingerprints)))
        .collect()
}
