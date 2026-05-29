use std::borrow::Cow;
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

/// A stable diagnostic category used by CLI and editor tooling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DiagnosticCategory {
    Freshness,
    Identifier,
    Markup,
    Metadata,
    Parse,
    Project,
    Schema,
    Validation,
    Unknown,
}

/// A stable diagnostic code, for example `RECITE_PARSE001`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticCode(Cow<'static, str>);

impl DiagnosticCode {
    pub fn new(value: impl Into<String>) -> Result<Self, CoreValueError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CoreValueError::EmptyDiagnosticCode);
        }

        if !is_namespaced_diagnostic_code(&value) {
            return Err(CoreValueError::NonNamespacedDiagnosticCode(value));
        }

        Ok(Self(Cow::Owned(value)))
    }

    #[must_use]
    pub const fn new_static(value: &'static str) -> Self {
        assert!(!value.is_empty(), "diagnostic code must not be empty");
        assert!(
            is_namespaced_diagnostic_code_const(value),
            "diagnostic code must be uppercase and namespaced"
        );

        Self(Cow::Borrowed(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn category(&self) -> DiagnosticCategory {
        diagnostic_category(self.0.as_bytes())
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
    pub fn error(code: DiagnosticCode, message: impl Into<String>, span: SourceSpan) -> Self {
        Self::new(code, DiagnosticSeverity::Error, message, span)
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

const fn is_namespaced_diagnostic_code_const(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut index = 0;
    let mut has_separator = false;
    let mut has_prefix = false;
    let mut has_code = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'_' {
            has_separator = true;
        } else if is_uppercase_letter_or_digit(byte) {
            if has_separator {
                has_code = true;
            } else {
                has_prefix = true;
            }
        } else {
            return false;
        }
        index += 1;
    }

    has_separator && has_prefix && has_code
}

const fn diagnostic_category(value: &[u8]) -> DiagnosticCategory {
    if starts_with(value, b"RECITE_ID") {
        DiagnosticCategory::Identifier
    } else if starts_with(value, b"RECITE_PARSE") {
        DiagnosticCategory::Parse
    } else if starts_with(value, b"RECITE_PROJECT") {
        DiagnosticCategory::Project
    } else if starts_with(value, b"RECITE_FRESH") {
        DiagnosticCategory::Freshness
    } else if starts_with(value, b"RECITE_SCHEMA") {
        DiagnosticCategory::Schema
    } else if starts_with(value, b"RECITE_VALIDATE") {
        validation_category(value)
    } else {
        DiagnosticCategory::Unknown
    }
}

const fn validation_category(value: &[u8]) -> DiagnosticCategory {
    let Some(number) = trailing_three_digit_number(value) else {
        return DiagnosticCategory::Validation;
    };

    if number >= 22 && number <= 25 {
        DiagnosticCategory::Markup
    } else if number >= 26 && number <= 30 {
        DiagnosticCategory::Metadata
    } else {
        DiagnosticCategory::Validation
    }
}

const fn trailing_three_digit_number(value: &[u8]) -> Option<u16> {
    if value.len() < 3 {
        return None;
    }

    let hundreds = value[value.len() - 3];
    let tens = value[value.len() - 2];
    let ones = value[value.len() - 1];
    if !is_digit(hundreds) || !is_digit(tens) || !is_digit(ones) {
        return None;
    }

    Some(((hundreds - b'0') as u16 * 100) + ((tens - b'0') as u16 * 10) + (ones - b'0') as u16)
}

const fn starts_with(value: &[u8], prefix: &[u8]) -> bool {
    if value.len() < prefix.len() {
        return false;
    }

    let mut index = 0;
    while index < prefix.len() {
        if value[index] != prefix[index] {
            return false;
        }
        index += 1;
    }

    true
}

const fn is_uppercase_letter_or_digit(byte: u8) -> bool {
    byte.is_ascii_uppercase() || is_digit(byte)
}

const fn is_digit(byte: u8) -> bool {
    byte >= b'0' && byte <= b'9'
}
