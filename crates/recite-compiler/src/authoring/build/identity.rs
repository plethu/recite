use recite_core::{
    COMPILER_COMPATIBILITY_VERSION_V0, ContentFingerprint, DocumentKey, ProjectSchema,
    SchemaFingerprint,
};

/// Build generations are independent from authoring snapshot generations.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BuildGeneration(pub(crate) u64);

impl BuildGeneration {
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
    pub fn next(self) -> Result<Self, BuildGenerationError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(BuildGenerationError::Exhausted { current: self })
    }
}

impl Default for BuildGeneration {
    fn default() -> Self {
        Self::initial()
    }
}

impl std::fmt::Display for BuildGeneration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildGenerationError {
    #[error("build generation {current} cannot advance further")]
    Exhausted { current: BuildGeneration },
}

/// The semantic family of one project input.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum BuildInputKind {
    Source,
    Schema,
    Manifest,
    Other(String),
}

/// Whether an input came from saved discovery or an editor overlay.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum BuildInputAuthority {
    Saved,
    Overlay,
}

/// Policy controlling whether unsaved overlays may participate.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildInputPolicy {
    #[default]
    SavedOnly,
    SavedAndOverlays,
}

/// A canonical input identity and content fingerprint.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildInput {
    pub(crate) key: DocumentKey,
    pub(crate) kind: BuildInputKind,
    pub(crate) authority: BuildInputAuthority,
    pub(crate) fingerprint: ContentFingerprint,
    pub(crate) content: String,
    pub(crate) schema: Option<ProjectSchema>,
}

impl BuildInput {
    #[must_use]
    pub fn new(
        key: DocumentKey,
        kind: BuildInputKind,
        authority: BuildInputAuthority,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        Self {
            key,
            kind,
            authority,
            fingerprint: recite_core::canonical_source_fingerprint(&content),
            content,
            schema: None,
        }
    }
    #[must_use]
    pub fn schema(
        key: DocumentKey,
        authority: BuildInputAuthority,
        source: impl Into<String>,
        model: ProjectSchema,
    ) -> Self {
        let content = source.into();
        Self {
            key,
            kind: BuildInputKind::Schema,
            authority,
            fingerprint: model.canonical_content_fingerprint(),
            content,
            schema: Some(model),
        }
    }
    #[must_use]
    pub fn saved_source(key: DocumentKey, source: impl AsRef<str>) -> Self {
        Self::new(
            key,
            BuildInputKind::Source,
            BuildInputAuthority::Saved,
            source.as_ref(),
        )
    }
    #[must_use]
    pub fn overlay_source(key: DocumentKey, source: impl AsRef<str>) -> Self {
        Self::new(
            key,
            BuildInputKind::Source,
            BuildInputAuthority::Overlay,
            source.as_ref(),
        )
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
    pub const fn authority(&self) -> BuildInputAuthority {
        self.authority
    }
    #[must_use]
    pub const fn fingerprint(&self) -> &ContentFingerprint {
        &self.fingerprint
    }
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
    #[must_use]
    pub const fn schema_model(&self) -> Option<&ProjectSchema> {
        self.schema.as_ref()
    }
}

/// A canonical fingerprint entry used by a publish guard.
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
                        .schema
                        .as_ref()
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
