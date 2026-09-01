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
    pub fn new(
        manifest: impl IntoIterator<Item = ProducerFingerprint>,
        registries: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
        metadata_domains: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
    ) -> Result<Self, ProducerFingerprintScopesError> {
        Ok(Self {
            manifest: sorted_manifest(manifest)?,
            registries: sorted_registries(registries)?,
            metadata_domains: sorted_metadata_domains(metadata_domains)?,
        })
    }

    pub fn from_schema(schema: &ProjectSchema) -> Result<Self, ProducerFingerprintScopesError> {
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
            ProducerFingerprintScopes::from_schema(schema)?,
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

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerLaunchSnapshotError {
    #[error("schema has no producer metadata")]
    MissingProducerMetadata,
    #[error("producer metadata has no producer identity")]
    MissingProducerIdentity,
    #[error("producer input fingerprints are invalid: {0}")]
    InvalidInputFingerprints(#[from] ProducerFingerprintScopesError),
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerFingerprintScopesError {
    #[error("manifest repeats fingerprint key '{kind}:{id}'")]
    DuplicateManifestKey { kind: String, id: String },
    #[error("registry scope name '{name}' is repeated")]
    DuplicateRegistryName { name: String },
    #[error("registry scope '{name}' repeats fingerprint key '{kind}:{id}'")]
    DuplicateRegistryKey {
        name: String,
        kind: String,
        id: String,
    },
    #[error("metadata-domain scope name '{name}' is repeated")]
    DuplicateMetadataDomainName { name: String },
    #[error("metadata-domain scope '{name}' repeats fingerprint key '{kind}:{id}'")]
    DuplicateMetadataDomainKey {
        name: String,
        kind: String,
        id: String,
    },
}

fn sorted_manifest(
    items: impl IntoIterator<Item = ProducerFingerprint>,
) -> Result<Vec<ProducerFingerprint>, ProducerFingerprintScopesError> {
    let mut items = items.into_iter().collect::<Vec<_>>();
    items.sort();
    for pair in items.windows(2) {
        if pair[0].kind == pair[1].kind && pair[0].id == pair[1].id {
            return Err(ProducerFingerprintScopesError::DuplicateManifestKey {
                kind: pair[0].kind.clone(),
                id: pair[0].id.clone(),
            });
        }
    }
    Ok(items)
}

fn sorted_registries(
    items: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
) -> Result<BTreeMap<String, Vec<ProducerFingerprint>>, ProducerFingerprintScopesError> {
    sorted_named(
        items,
        |name| ProducerFingerprintScopesError::DuplicateRegistryName { name },
        |name, fingerprint| ProducerFingerprintScopesError::DuplicateRegistryKey {
            name,
            kind: fingerprint.kind.clone(),
            id: fingerprint.id.clone(),
        },
    )
}

fn sorted_metadata_domains(
    items: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
) -> Result<BTreeMap<String, Vec<ProducerFingerprint>>, ProducerFingerprintScopesError> {
    sorted_named(
        items,
        |name| ProducerFingerprintScopesError::DuplicateMetadataDomainName { name },
        |name, fingerprint| ProducerFingerprintScopesError::DuplicateMetadataDomainKey {
            name,
            kind: fingerprint.kind.clone(),
            id: fingerprint.id.clone(),
        },
    )
}

fn sorted_named(
    items: impl IntoIterator<Item = (String, Vec<ProducerFingerprint>)>,
    duplicate_name: impl Fn(String) -> ProducerFingerprintScopesError,
    duplicate_key: impl Fn(String, &ProducerFingerprint) -> ProducerFingerprintScopesError,
) -> Result<BTreeMap<String, Vec<ProducerFingerprint>>, ProducerFingerprintScopesError> {
    let mut result = BTreeMap::new();
    for (name, mut fingerprints) in items {
        if result.contains_key(&name) {
            return Err(duplicate_name(name));
        }
        fingerprints.sort();
        for pair in fingerprints.windows(2) {
            if pair[0].kind == pair[1].kind && pair[0].id == pair[1].id {
                return Err(duplicate_key(name, &pair[0]));
            }
        }
        result.insert(name, fingerprints);
    }
    Ok(result)
}
