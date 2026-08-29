use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use super::diagnostic::DiagnosticArgumentType;

/// An error constructing a locale-neutral diagnostic presentation.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[non_exhaustive]
pub enum DiagnosticPresentationError {
    #[error(
        "invalid diagnostic presentation ID `{0}`; IDs must start with a lowercase ASCII letter and contain only lowercase ASCII letters, digits, or single '-' separators"
    )]
    InvalidId(String),
    #[error(
        "invalid diagnostic argument name `{0}`; names must start with a lowercase ASCII letter and contain only lowercase ASCII letters, digits, or '_'"
    )]
    InvalidArgumentName(String),
    #[error("diagnostic argument `{0}` was provided more than once")]
    DuplicateArgument(String),
    #[error("diagnostic argument floats must be finite")]
    NonFiniteFloat,
    #[error(
        "no diagnostic presentation contract exists for code `{code}` and ID `{presentation_id}`"
    )]
    UnknownContract {
        code: String,
        presentation_id: String,
    },
    #[error("diagnostic argument `{0}` is required by the presentation contract")]
    MissingArgument(String),
    #[error("diagnostic argument `{0}` is not declared by the presentation contract")]
    ExtraArgument(String),
    #[error(
        "diagnostic argument `{name}` has type {actual:?}; the presentation contract requires {expected:?}"
    )]
    ArgumentTypeMismatch {
        name: String,
        expected: DiagnosticArgumentType,
        actual: DiagnosticArgumentType,
    },
}

/// A stable resource identifier for one diagnostic presentation.
///
/// This is deliberately independent of Fluent. A later client or inventory
/// layer may resolve this ID through Fluent, another resource format, or a
/// generated catalogue without changing the core diagnostic contract.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct DiagnosticPresentationId(Cow<'static, str>);

impl DiagnosticPresentationId {
    /// Construct an owned presentation ID after validating its stable shape.
    pub fn new(value: impl Into<String>) -> Result<Self, DiagnosticPresentationError> {
        let value = value.into();
        if !is_valid_presentation_id(&value) {
            return Err(DiagnosticPresentationError::InvalidId(value));
        }

        Ok(Self(Cow::Owned(value)))
    }

    /// Construct a validated borrowed ID for static diagnostic inventories.
    #[must_use]
    pub const fn new_static(value: &'static str) -> Self {
        assert!(
            is_valid_presentation_id_const(value),
            "diagnostic presentation ID has an invalid stable shape"
        );
        Self(Cow::Borrowed(value))
    }

    pub(crate) fn from_validated(value: String) -> Self {
        Self(Cow::Owned(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticPresentationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DiagnosticPresentationId {
    type Error = DiagnosticPresentationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DiagnosticPresentationId {
    type Error = DiagnosticPresentationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for DiagnosticPresentationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticPresentationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

fn is_valid_presentation_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }

    let mut previous_separator = false;
    for byte in bytes.iter().copied() {
        let is_separator = byte == b'-';
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || is_separator)
            || (is_separator && previous_separator)
        {
            return false;
        }
        previous_separator = is_separator;
    }

    !previous_separator
}

const fn is_valid_presentation_id_const(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || !is_lowercase_letter(bytes[0]) {
        return false;
    }

    let mut index = 0;
    let mut previous_separator = false;
    while index < bytes.len() {
        let byte = bytes[index];
        let is_separator = byte == b'-';
        if !(is_lowercase_letter(byte) || is_digit(byte) || is_separator)
            || (is_separator && previous_separator)
        {
            return false;
        }
        previous_separator = is_separator;
        index += 1;
    }

    !previous_separator
}

const fn is_lowercase_letter(byte: u8) -> bool {
    byte.is_ascii_lowercase()
}

const fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}
