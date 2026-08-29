use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable identity of the host producer that owns a generated manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProducerIdentity {
    kind: String,
    id: String,
}

impl ProducerIdentity {
    /// Creates a validated producer identity without imposing a character
    /// grammar beyond the non-empty/whitespace contract.
    pub fn new(
        kind: impl Into<String>,
        id: impl Into<String>,
    ) -> Result<Self, ProducerIdentityError> {
        let kind = kind.into();
        validate_identity_part(ProducerIdentityPart::Kind, &kind)?;
        let id = id.into();
        validate_identity_part(ProducerIdentityPart::Id, &id)?;
        Ok(Self { kind, id })
    }

    /// Returns the producer kind.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the producer identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Which producer identity component failed validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
#[non_exhaustive]
pub enum ProducerIdentityPart {
    /// Producer kind.
    #[error("producer kind")]
    Kind,
    /// Producer identifier.
    #[error("producer id")]
    Id,
}

/// Failure constructing a producer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
#[non_exhaustive]
pub enum ProducerIdentityError {
    /// A component was empty.
    #[error("{part} must not be empty")]
    Empty { part: ProducerIdentityPart },
    /// A component contained only whitespace.
    #[error("{part} must not contain only whitespace")]
    Whitespace { part: ProducerIdentityPart },
}

fn validate_identity_part(
    part: ProducerIdentityPart,
    value: &str,
) -> Result<(), ProducerIdentityError> {
    if value.is_empty() {
        return Err(ProducerIdentityError::Empty { part });
    }
    if value.trim().is_empty() {
        return Err(ProducerIdentityError::Whitespace { part });
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProducerIdentityWire {
    kind: String,
    id: String,
}

impl<'de> Deserialize<'de> for ProducerIdentity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProducerIdentityWire::deserialize(deserializer)?;
        Self::new(wire.kind, wire.id).map_err(serde::de::Error::custom)
    }
}
