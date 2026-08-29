use std::collections::BTreeMap;

use fluent_bundle::{FluentArgs, FluentValue};

// Fluent numbers are backed by f64. Keep selectors numeric while every value
// is exactly representable, and switch to a string interpolation for larger
// integers so the typed i64 value is never rounded at the UI boundary.
const MAX_EXACT_FLUENT_INTEGER: i64 = 1_i64 << 53;

/// The supported value kinds at the UI boundary. Values are intentionally
/// format-neutral; Fluent is an implementation detail of the resolver.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum UiArg {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl UiArg {
    pub fn kind(&self) -> UiArgType {
        match self {
            Self::String(_) => UiArgType::String,
            Self::Integer(_) => UiArgType::Integer,
            Self::Float(_) => UiArgType::Float,
            Self::Boolean(_) => UiArgType::Boolean,
        }
    }

    pub(crate) fn fluent_value(&self) -> FluentValue<'static> {
        match self {
            Self::String(value) => FluentValue::String(value.clone().into()),
            Self::Integer(value)
                if *value >= -MAX_EXACT_FLUENT_INTEGER && *value <= MAX_EXACT_FLUENT_INTEGER =>
            {
                FluentValue::Number((*value as f64).into())
            }
            Self::Integer(value) => FluentValue::String(value.to_string().into()),
            Self::Float(value) => FluentValue::Number((*value).into()),
            Self::Boolean(value) => {
                FluentValue::String(if *value { "true" } else { "false" }.into())
            }
        }
    }
}

impl std::fmt::Display for UiArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(value) => formatter.write_str(value),
            Self::Integer(value) => value.fmt(formatter),
            Self::Float(value) => value.fmt(formatter),
            Self::Boolean(value) => value.fmt(formatter),
        }
    }
}

impl From<String> for UiArg {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for UiArg {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<&String> for UiArg {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl From<i64> for UiArg {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for UiArg {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<usize> for UiArg {
    fn from(value: usize) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<u32> for UiArg {
    fn from(value: u32) -> Self {
        Self::Integer(i64::from(value))
    }
}

impl From<bool> for UiArg {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

/// Deterministically ordered named arguments.
pub type UiArgs = BTreeMap<String, UiArg>;

/// Argument kinds declared by a resource contract.
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
pub enum UiArgType {
    String,
    Integer,
    Float,
    Boolean,
}

pub(crate) fn fluent_args(args: &UiArgs) -> FluentArgs<'_> {
    let mut fluent = FluentArgs::new();
    for (name, value) in args {
        fluent.set(name, value.fluent_value());
    }
    fluent
}
