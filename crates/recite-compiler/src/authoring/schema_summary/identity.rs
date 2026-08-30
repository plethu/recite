use recite_core::{
    ContentFingerprint, ProducerFingerprint, ProducerFreshness, ProducerIdentity, ProducerOrigin,
    SchemaFingerprint,
};

/// Identifies who owns the declarations represented by a schema summary.
///
/// A missing producer is deliberately not treated as a standalone source. The
/// canonical model can be constructed without producer metadata, so clients
/// must see that ownership is unavailable instead of being given an inferred
/// source path or command.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaOwnership {
    Standalone { producer: ProducerIdentity },
    Generated { producer: ProducerIdentity },
    Unavailable,
}

impl SchemaOwnership {
    #[must_use]
    pub fn producer(&self) -> Option<&ProducerIdentity> {
        match self {
            Self::Standalone { producer } | Self::Generated { producer } => Some(producer),
            Self::Unavailable => None,
        }
    }

    #[must_use]
    pub const fn is_standalone(&self) -> bool {
        matches!(self, Self::Standalone { .. })
    }

    #[must_use]
    pub const fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }
}

/// Provenance for one canonical declaration.
///
/// `origin` is present only where the canonical schema carries a declaration
/// or value origin. It is not synthesised from a debug representation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaDeclarationProvenance {
    pub(super) ownership: SchemaOwnership,
    pub(super) origin: Option<ProducerOrigin>,
}

impl SchemaDeclarationProvenance {
    #[must_use]
    pub const fn ownership(&self) -> &SchemaOwnership {
        &self.ownership
    }

    #[must_use]
    pub fn origin(&self) -> Option<&ProducerOrigin> {
        self.origin.as_ref()
    }
}

/// A source-owning or producer-backed action boundary exposed to clients.
///
/// These are descriptors only. The compiler does not open files, invoke
/// producers, retry failures, or write generated manifests.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaAction {
    OpenSourceDeclaration,
    EditStandaloneSource,
    InvokeProducer {
        producer: ProducerIdentity,
    },
    RetryProducerFailure {
        producer: ProducerIdentity,
    },
    ReadOnlyGenerated,
    Unavailable {
        reason: SchemaCapabilityUnavailableReason,
    },
}

/// Why a schema action cannot be offered from canonical inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaCapabilityUnavailableReason {
    UnknownSourceOwner,
}

/// The action descriptors available for a schema or declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaCapability {
    pub(super) actions: Vec<SchemaAction>,
}

impl SchemaCapability {
    #[must_use]
    pub fn actions(&self) -> &[SchemaAction] {
        &self.actions
    }

    #[must_use]
    pub fn supports(&self, action: &SchemaAction) -> bool {
        self.actions.iter().any(|candidate| candidate == action)
    }

    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.actions
            .iter()
            .any(|action| matches!(action, SchemaAction::ReadOnlyGenerated))
    }
}

/// Freshness comparison state for producer-owned content.
///
/// A single `ProjectSchema` contains evidence, but not an expected/current
/// pair to compare. Such a summary therefore reports `Unavailable` until a
/// caller supplies the comparison snapshots to the canonical core API.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaFreshness {
    Compared(ProducerFreshness),
    Unavailable {
        reason: SchemaFreshnessUnavailableReason,
    },
}

/// Why freshness could not be evaluated for one summary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaFreshnessUnavailableReason {
    NoComparisonSnapshot,
    NoProducerMetadata,
}

/// Fingerprint evidence retained by the authoring summary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaFingerprintSummary {
    pub(super) semantic: SchemaFingerprint,
    pub(super) canonical_content: ContentFingerprint,
    pub(super) source_owned: Option<ContentFingerprint>,
    pub(super) producer_content: Option<ContentFingerprint>,
    pub(super) producer_inputs: Vec<ProducerFingerprint>,
}

impl SchemaFingerprintSummary {
    #[must_use]
    pub const fn semantic(&self) -> &SchemaFingerprint {
        &self.semantic
    }

    #[must_use]
    pub const fn canonical_content(&self) -> &ContentFingerprint {
        &self.canonical_content
    }

    #[must_use]
    pub const fn source_owned(&self) -> Option<&ContentFingerprint> {
        self.source_owned.as_ref()
    }

    #[must_use]
    pub const fn producer_content(&self) -> Option<&ContentFingerprint> {
        self.producer_content.as_ref()
    }

    #[must_use]
    pub fn producer_inputs(&self) -> &[ProducerFingerprint] {
        &self.producer_inputs
    }
}

/// Producer metadata and freshness evidence copied from the canonical model.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerMetadataSummary {
    pub(super) producer: Option<ProducerIdentity>,
    pub(super) content_fingerprint: Option<ContentFingerprint>,
    pub(super) schema_export_version: Option<u32>,
    pub(super) inclusion_policy: Option<String>,
    pub(super) producer_fingerprints: Vec<ProducerFingerprint>,
    pub(super) freshness: SchemaFreshness,
}

impl ProducerMetadataSummary {
    #[must_use]
    pub const fn producer(&self) -> Option<&ProducerIdentity> {
        self.producer.as_ref()
    }

    #[must_use]
    pub const fn content_fingerprint(&self) -> Option<&ContentFingerprint> {
        self.content_fingerprint.as_ref()
    }

    #[must_use]
    pub const fn schema_export_version(&self) -> Option<u32> {
        self.schema_export_version
    }

    #[must_use]
    pub fn inclusion_policy(&self) -> Option<&str> {
        self.inclusion_policy.as_deref()
    }

    #[must_use]
    pub fn producer_fingerprints(&self) -> &[ProducerFingerprint] {
        &self.producer_fingerprints
    }

    #[must_use]
    pub const fn freshness(&self) -> &SchemaFreshness {
        &self.freshness
    }
}
