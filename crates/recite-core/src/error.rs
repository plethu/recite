use std::fmt;

/// Error returned when constructing constrained core model values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreValueError {
    ZeroSourceLine,
    ZeroSourceColumn,
    EmptyDiagnosticCode,
    NonNamespacedDiagnosticCode(String),
    EmptyId { kind: &'static str },
}

impl fmt::Display for CoreValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSourceLine => formatter.write_str("source line must be 1-based"),
            Self::ZeroSourceColumn => formatter.write_str("source column must be 1-based"),
            Self::EmptyDiagnosticCode => formatter.write_str("diagnostic code must not be empty"),
            Self::NonNamespacedDiagnosticCode(code) => {
                write!(formatter, "diagnostic code `{code}` must be namespaced")
            }
            Self::EmptyId { kind } => write!(formatter, "{kind} must not be empty"),
        }
    }
}

impl std::error::Error for CoreValueError {}
