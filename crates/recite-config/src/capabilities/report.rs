use std::collections::BTreeMap;

use recite_core::{DiagnosticRecord, ProducerIdentity};
use serde::{Deserialize, Deserializer, Serialize};

use super::name::CapabilityName;

/// Version of the shared capability report contract.
pub const CAPABILITY_REPORT_VERSION: u16 = 1;

/// Availability advertised for one capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityStatusWire {
    status: String,
    #[serde(default)]
    diagnostic: Option<Box<DiagnosticRecord>>,
}

impl<'de> Deserialize<'de> for CapabilityStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityStatusWire::deserialize(deserializer)?;
        match (wire.status.as_str(), wire.diagnostic) {
            ("supported", None) => Ok(Self::Supported),
            ("read_only", None) => Ok(Self::ReadOnly),
            ("unavailable", Some(diagnostic)) => Ok(Self::Unavailable { diagnostic }),
            ("supported" | "read_only", Some(_)) => Err(serde::de::Error::custom(
                "supported capability statuses cannot include a diagnostic",
            )),
            ("unavailable", None) => Err(serde::de::Error::custom(
                "unavailable capability status requires a diagnostic",
            )),
            (status, _) => Err(serde::de::Error::custom(format!(
                "unknown capability status {status:?}"
            ))),
        }
    }
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
#[serde(deny_unknown_fields)]
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
            if let Some(existing) = unique.get(entry.name()) {
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
