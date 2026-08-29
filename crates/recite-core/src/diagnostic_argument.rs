use std::hash::{Hash, Hasher};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::diagnostic::DiagnosticArgumentType;
use super::diagnostic_presentation::DiagnosticPresentationError;

/// A finite floating-point value safe to carry in a diagnostic argument.
#[derive(Clone, Copy, Debug)]
pub struct DiagnosticFiniteFloat(f64);

impl DiagnosticFiniteFloat {
    /// Construct a finite floating-point value.
    pub fn new(value: f64) -> Result<Self, DiagnosticPresentationError> {
        if !value.is_finite() {
            return Err(DiagnosticPresentationError::NonFiniteFloat);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for DiagnosticFiniteFloat {
    type Error = DiagnosticPresentationError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DiagnosticFiniteFloat> for f64 {
    fn from(value: DiagnosticFiniteFloat) -> Self {
        value.as_f64()
    }
}

impl PartialEq for DiagnosticFiniteFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for DiagnosticFiniteFloat {}

impl Hash for DiagnosticFiniteFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Serialize for DiagnosticFiniteFloat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for DiagnosticFiniteFloat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Typed values allowed in a diagnostic presentation's named arguments.
///
/// Floats are retained for diagnostics that report measured values, but the
/// finite wrapper makes NaN and infinities unrepresentable in this public type.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum DiagnosticArgumentValue {
    String(String),
    Integer(i64),
    Float(DiagnosticFiniteFloat),
    Boolean(bool),
}

impl DiagnosticArgumentValue {
    /// Construct a finite floating-point argument.
    pub fn try_float(value: f64) -> Result<Self, DiagnosticPresentationError> {
        Ok(Self::Float(DiagnosticFiniteFloat::new(value)?))
    }

    pub(crate) fn validate(&self) -> Result<(), DiagnosticPresentationError> {
        // `DiagnosticFiniteFloat` makes the finite invariant unrepresentable
        // through the public argument enum.
        Ok(())
    }

    #[must_use]
    pub(crate) fn argument_type(&self) -> DiagnosticArgumentType {
        match self {
            Self::String(_) => DiagnosticArgumentType::String,
            Self::Integer(_) => DiagnosticArgumentType::Integer,
            Self::Float(_) => DiagnosticArgumentType::Float,
            Self::Boolean(_) => DiagnosticArgumentType::Boolean,
        }
    }
}

impl PartialEq for DiagnosticArgumentValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for DiagnosticArgumentValue {}

impl Hash for DiagnosticArgumentValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::String(value) => value.hash(state),
            Self::Integer(value) => value.hash(state),
            Self::Float(value) => value.hash(state),
            Self::Boolean(value) => value.hash(state),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum DiagnosticArgumentValueWire {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl Serialize for DiagnosticArgumentValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let wire = match self {
            Self::String(value) => DiagnosticArgumentValueWire::String(value.clone()),
            Self::Integer(value) => DiagnosticArgumentValueWire::Integer(*value),
            Self::Float(value) => DiagnosticArgumentValueWire::Float(value.as_f64()),
            Self::Boolean(value) => DiagnosticArgumentValueWire::Boolean(*value),
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiagnosticArgumentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = match DiagnosticArgumentValueWire::deserialize(deserializer)? {
            DiagnosticArgumentValueWire::String(value) => Self::String(value),
            DiagnosticArgumentValueWire::Integer(value) => Self::Integer(value),
            DiagnosticArgumentValueWire::Float(value) => {
                Self::Float(DiagnosticFiniteFloat::new(value).map_err(serde::de::Error::custom)?)
            }
            DiagnosticArgumentValueWire::Boolean(value) => Self::Boolean(value),
        };
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}
