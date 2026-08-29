use std::{collections::BTreeMap, fmt, str::FromStr};

use recite_core::{DiagnosticRecord, ProducerIdentity};
use serde::{Deserialize, Deserializer, Serialize};

/// Version of the shared capability report contract.
pub const CAPABILITY_REPORT_VERSION: u16 = 1;

/// A validated, namespaced capability name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CapabilityName(String);

/// Alias emphasizing that a capability name is an authoring-visible ID.
pub type CapabilityId = CapabilityName;

impl CapabilityName {
    /// Parses a lower-case dotted namespace such as `recite.cli.compile`.
    pub fn new(value: impl Into<String>) -> Result<Self, CapabilityNameError> {
        let value = value.into();
        if value.is_empty() || value.trim() != value {
            return Err(CapabilityNameError::Invalid {
                value,
                reason: "name must be non-empty and have no surrounding whitespace",
            });
        }
        let mut segments = value.split('.');
        let valid = segments.clone().count() >= 2
            && segments.all(|segment| {
                !segment.is_empty()
                    && segment.chars().all(|character| {
                        character.is_ascii_lowercase()
                            || character.is_ascii_digit()
                            || character == '-'
                            || character == '_'
                    })
                    && segment
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_lowercase())
            });
        if !valid {
            return Err(CapabilityNameError::Invalid {
                value,
                reason: "name must contain at least two lower-case dotted segments",
            });
        }
        Ok(Self(value))
    }

    /// Returns the stable name without allocation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapabilityName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CapabilityName {
    type Err = CapabilityNameError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for CapabilityName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Failure to construct a namespaced capability name.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CapabilityNameError {
    /// The name did not satisfy the stable namespace grammar.
    #[error("invalid capability name {value:?}: {reason}")]
    Invalid {
        /// Original name supplied by the caller.
        value: String,
        /// Stable validation reason.
        reason: &'static str,
    },
}

/// Availability advertised for one capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityStatus {
    /// The producer supports the capability.
    Supported,
    /// The producer exposes the capability for inspection but cannot edit it.
    ReadOnly,
    /// The capability is known but unavailable, with a structured diagnostic
    /// record for the caller's presentation layer and durable evidence.
    Unavailable {
        /// Structured diagnostic explaining unavailability and its context.
        diagnostic: Box<DiagnosticRecord>,
    },
}

impl CapabilityStatus {
    /// Creates an unavailable status with recordable structured context.
    #[must_use]
    pub fn unavailable(diagnostic: DiagnosticRecord) -> Self {
        Self::Unavailable {
            diagnostic: Box::new(diagnostic),
        }
    }

    /// Returns the structured diagnostic for an unavailable capability.
    #[must_use]
    pub const fn diagnostic(&self) -> Option<&DiagnosticRecord> {
        match self {
            Self::Unavailable { diagnostic } => Some(diagnostic),
            Self::Supported | Self::ReadOnly => None,
        }
    }
}

/// One deterministic capability entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Capability {
    /// Stable namespaced capability ID.
    name: CapabilityName,
    /// Availability state.
    status: CapabilityStatus,
}

impl Capability {
    /// Creates an entry from an already validated name.
    #[must_use]
    pub const fn new(name: CapabilityName, status: CapabilityStatus) -> Self {
        Self { name, status }
    }

    /// Returns the stable capability ID.
    #[must_use]
    pub const fn name(&self) -> &CapabilityName {
        &self.name
    }

    /// Returns the advertised availability.
    #[must_use]
    pub const fn status(&self) -> &CapabilityStatus {
        &self.status
    }
}

/// Versioned, producer-owned capability data for shared authoring surfaces.
///
/// This is deliberately not an LSP `ServerCapabilities` projection. It is a
/// small local contract that can be consumed by CLI, LSP, GUI, and adapters
/// without making any one transport authoritative.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct CapabilityReport {
    /// Wire/report contract version.
    version: u16,
    /// Typed identity of the producer that owns this report.
    producer: ProducerIdentity,
    /// Sorted and de-duplicated capabilities.
    capabilities: Vec<Capability>,
}

impl CapabilityReport {
    /// Builds a report, rejecting conflicting duplicate names.
    pub fn new(
        producer: ProducerIdentity,
        entries: impl IntoIterator<Item = Capability>,
    ) -> Result<Self, CapabilityReportError> {
        let mut unique = BTreeMap::<CapabilityName, CapabilityStatus>::new();
        for entry in entries {
            if let Some(existing) = unique.get(&entry.name) {
                if existing != entry.status() {
                    return Err(CapabilityReportError::ConflictingDuplicate {
                        name: entry.name,
                        existing: Box::new(existing.clone()),
                        incoming: Box::new(entry.status),
                    });
                }
                continue;
            }
            unique.insert(entry.name, entry.status);
        }
        let capabilities = unique
            .into_iter()
            .map(|(name, status)| Capability { name, status })
            .collect();
        Ok(Self {
            version: CAPABILITY_REPORT_VERSION,
            producer,
            capabilities,
        })
    }

    /// Returns the report wire version.
    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Returns the typed producer identity.
    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    /// Returns sorted, de-duplicated capability entries.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Returns whether a capability is advertised as supported.
    #[must_use]
    pub fn supports(&self, name: &CapabilityName) -> bool {
        self.capabilities
            .binary_search_by(|entry| entry.name.cmp(name))
            .is_ok_and(|index| self.capabilities[index].status == CapabilityStatus::Supported)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityReportWire {
    version: u16,
    producer: ProducerIdentity,
    capabilities: Vec<Capability>,
}

impl<'de> Deserialize<'de> for CapabilityReport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityReportWire::deserialize(deserializer)?;
        if wire.version != CAPABILITY_REPORT_VERSION {
            return Err(serde::de::Error::custom(
                CapabilityReportError::UnsupportedVersion {
                    found: wire.version,
                    expected: CAPABILITY_REPORT_VERSION,
                },
            ));
        }
        Self::new(wire.producer, wire.capabilities).map_err(serde::de::Error::custom)
    }
}

/// Failure while constructing a deterministic capability report.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CapabilityReportError {
    /// The same capability was supplied with incompatible availability states.
    #[error("capability {name} was supplied with conflicting statuses")]
    ConflictingDuplicate {
        /// Conflicting capability name.
        name: CapabilityName,
        /// First status observed.
        existing: Box<CapabilityStatus>,
        /// Later status observed.
        incoming: Box<CapabilityStatus>,
    },
    /// A report used a wire version this crate cannot interpret.
    #[error("unsupported capability report version {found}; expected {expected}")]
    UnsupportedVersion {
        /// Version found on the wire.
        found: u16,
        /// Only version currently understood by this crate.
        expected: u16,
    },
}
