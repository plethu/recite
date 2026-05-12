use std::fmt;

use crate::{CoreValueError, SourceSpan};

/// Stable diagnostic severity shared by compiler, CLI, and LSP surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A stable diagnostic code, for example `RECITE_PARSE001`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CoreValueError::EmptyDiagnosticCode);
        }

        if !is_namespaced_diagnostic_code(&value) {
            return Err(CoreValueError::NonNamespacedDiagnosticCode(value));
        }

        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<&str> for DiagnosticCode {
    type Error = CoreValueError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DiagnosticCode {
    type Error = CoreValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Additional source location related to a primary diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RelatedSpan {
    pub span: SourceSpan,
    pub message: String,
}

impl RelatedSpan {
    #[must_use]
    pub fn new(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// A structured diagnostic that can be rendered by CLI and editor tooling.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: SourceSpan,
    pub related: Vec<RelatedSpan>,
    pub help: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self {
            code,
            severity,
            message: message.into(),
            span,
            related: Vec::new(),
            help: None,
        }
    }

    #[must_use]
    pub fn with_related(mut self, related: impl IntoIterator<Item = RelatedSpan>) -> Self {
        self.related.extend(related);
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

fn is_namespaced_diagnostic_code(value: &str) -> bool {
    let Some((namespace, code)) = value.split_once('_') else {
        return false;
    };

    !namespace.is_empty()
        && !code.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
}
