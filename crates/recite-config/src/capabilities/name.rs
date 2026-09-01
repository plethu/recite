use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};

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
