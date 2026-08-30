use super::identity::{BuildInput, BuildInputKind};
use recite_core::{
    COMPILER_COMPATIBILITY_VERSION_V0, ContentFingerprint, DocumentKey, ProjectSchema,
    SchemaFingerprint,
};

/// A canonical fingerprint entry used by a publication guard.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildInputFingerprint {
    key: DocumentKey,
    kind: BuildInputKind,
    fingerprint: ContentFingerprint,
}
impl BuildInputFingerprint {
    pub(crate) fn from_input(input: &BuildInput) -> Self {
        Self {
            key: input.key.clone(),
            kind: input.kind.clone(),
            fingerprint: input.fingerprint.clone(),
        }
    }
    #[must_use]
    pub const fn key(&self) -> &DocumentKey {
        &self.key
    }
    #[must_use]
    pub const fn kind(&self) -> &BuildInputKind {
        &self.kind
    }
    #[must_use]
    pub const fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }
}

/// Source, schema, and compiler values captured by one request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildFingerprintSet {
    inputs: Vec<BuildInputFingerprint>,
    schema: SchemaFingerprint,
    compiler_compatibility_version: u16,
}
impl BuildFingerprintSet {
    pub(crate) fn from_inputs(inputs: &[BuildInput]) -> Self {
        let mut entries = inputs
            .iter()
            .map(BuildInputFingerprint::from_input)
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.key.cmp(&right.key).then(left.kind.cmp(&right.kind)));
        Self {
            inputs: entries,
            schema: inputs
                .iter()
                .find_map(|input| {
                    input
                        .schema_model()
                        .map(ProjectSchema::canonical_fingerprint)
                })
                .unwrap_or(SchemaFingerprint::NoSchema),
            compiler_compatibility_version: COMPILER_COMPATIBILITY_VERSION_V0,
        }
    }
    #[must_use]
    pub fn inputs(&self) -> &[BuildInputFingerprint] {
        &self.inputs
    }
    #[must_use]
    pub const fn schema(&self) -> &SchemaFingerprint {
        &self.schema
    }
    #[must_use]
    pub const fn compiler_compatibility_version(&self) -> u16 {
        self.compiler_compatibility_version
    }
}
pub(crate) fn default_fingerprints(inputs: &[BuildInput]) -> BuildFingerprintSet {
    BuildFingerprintSet::from_inputs(inputs)
}
