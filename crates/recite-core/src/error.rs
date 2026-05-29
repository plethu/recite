/// Error returned when constructing constrained core model values.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CoreValueError {
    #[error("source line must be 1-based")]
    ZeroSourceLine,
    #[error("source column must be 1-based")]
    ZeroSourceColumn,
    #[error("diagnostic code must not be empty")]
    EmptyDiagnosticCode,
    #[error("diagnostic code `{0}` must be namespaced")]
    NonNamespacedDiagnosticCode(String),
    #[error("{kind} must not be empty")]
    EmptyId { kind: &'static str },
}
