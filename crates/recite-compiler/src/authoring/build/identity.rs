use recite_core::{ContentFingerprint, DocumentKey, ProjectSchema};

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

/// Immutable compiler-owned payload for one discovered input.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BuildInputPayload {
    Text(String),
    Schema(Box<ProjectSchema>),
}
impl From<String> for BuildInputPayload {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<&str> for BuildInputPayload {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

/// A canonical input identity and content fingerprint.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildInput {
    pub(crate) key: DocumentKey,
    pub(crate) kind: BuildInputKind,
    pub(crate) authority: BuildInputAuthority,
    pub(crate) payload: BuildInputPayload,
    pub(crate) fingerprint: ContentFingerprint,
}

impl BuildInput {
    #[must_use]
    pub fn new(
        key: DocumentKey,
        kind: BuildInputKind,
        authority: BuildInputAuthority,
        payload: impl Into<BuildInputPayload>,
    ) -> Self {
        let payload = payload.into();
        let fingerprint = match &payload {
            BuildInputPayload::Text(content) => recite_core::canonical_source_fingerprint(content),
            BuildInputPayload::Schema(model) => model.canonical_content_fingerprint(),
        };
        Self {
            key,
            kind,
            authority,
            payload,
            fingerprint,
        }
    }
    #[must_use]
    pub fn schema(key: DocumentKey, authority: BuildInputAuthority, model: ProjectSchema) -> Self {
        Self::new(
            key,
            BuildInputKind::Schema,
            authority,
            BuildInputPayload::Schema(Box::new(model)),
        )
    }
    #[must_use]
    pub fn saved_source(key: DocumentKey, source: impl AsRef<str>) -> Self {
        Self::new(
            key,
            BuildInputKind::Source,
            BuildInputAuthority::Saved,
            BuildInputPayload::Text(source.as_ref().to_owned()),
        )
    }
    #[must_use]
    pub fn overlay_source(key: DocumentKey, source: impl AsRef<str>) -> Self {
        Self::new(
            key,
            BuildInputKind::Source,
            BuildInputAuthority::Overlay,
            BuildInputPayload::Text(source.as_ref().to_owned()),
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
    pub fn payload(&self) -> &BuildInputPayload {
        &self.payload
    }
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        match &self.payload {
            BuildInputPayload::Text(content) => Some(content),
            BuildInputPayload::Schema(_) => None,
        }
    }
    #[must_use]
    pub fn schema_model(&self) -> Option<&ProjectSchema> {
        match &self.payload {
            BuildInputPayload::Schema(model) => Some(model),
            BuildInputPayload::Text(_) => None,
        }
    }
}
