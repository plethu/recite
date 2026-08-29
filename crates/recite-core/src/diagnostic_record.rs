use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::{DiagnosticCode, DiagnosticSeverity, RelatedSpan, SourcePosition, SourceSpan};

use crate::diagnostic_presentation_guidance::{
    DiagnosticExplanationPresentation, DiagnosticRelatedPresentation,
};
use crate::diagnostic_presentation_record::DiagnosticPresentation;

/// Current wire version for [`DiagnosticRecord`].
pub const DIAGNOSTIC_RECORD_VERSION: u16 = 1;

/// Failure converting a legacy [`crate::Diagnostic`] to a structured record.
#[derive(Clone, Debug, Eq, PartialEq, Error)]
#[non_exhaustive]
pub enum DiagnosticRecordError {
    #[error("diagnostic has no structured primary presentation")]
    MissingPresentation,
    #[error("diagnostic still contains legacy related/help context")]
    LegacyContext {
        /// Legacy related spans in their original source order.
        related: Vec<RelatedSpan>,
        /// Legacy help text, when supplied.
        help: Option<String>,
    },
}

/// The structured, locale-neutral form of a diagnostic.
///
/// This record is authoritative for new producers. It intentionally has no
/// rendered message field; `compatibility_message` is only an explicitly
/// supplied deterministic en-US fallback for clients that cannot resolve the
/// presentation. The existing [`crate::Diagnostic::message`] field is copied
/// into that fallback only at the legacy-wrapper conversion boundary.
///
/// Version 1 is a closed wire shape: deserialization rejects unsupported
/// versions, unknown fields, and duplicate named arguments. A future version
/// must define an explicit migration before changing this shape.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
#[non_exhaustive]
pub struct DiagnosticRecord {
    version: u16,
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub span: SourceSpan,
    pub presentation: DiagnosticPresentation,
    pub related: Vec<DiagnosticRelatedPresentation>,
    pub help: Option<DiagnosticPresentation>,
    pub explanation: Option<DiagnosticExplanationPresentation>,
    compatibility_message: Option<String>,
}

impl DiagnosticRecord {
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        severity: DiagnosticSeverity,
        span: SourceSpan,
        presentation: DiagnosticPresentation,
    ) -> Self {
        Self {
            version: DIAGNOSTIC_RECORD_VERSION,
            code,
            severity,
            span,
            presentation,
            related: Vec::new(),
            help: None,
            explanation: None,
            compatibility_message: None,
        }
    }

    #[must_use]
    pub const fn version(&self) -> u16 {
        self.version
    }

    /// Store the deterministic en-US fallback supplied by a compatibility
    /// wrapper. This does not replace the authoritative structured data.
    #[must_use]
    pub fn with_compatibility_message(mut self, message: impl Into<String>) -> Self {
        self.compatibility_message = Some(message.into());
        self
    }

    #[must_use]
    pub fn compatibility_message(&self) -> Option<&str> {
        self.compatibility_message.as_deref()
    }

    /// Prefer client-resolved text and fall back to the explicitly supplied
    /// deterministic en-US compatibility message.
    #[must_use]
    pub fn message_or<'a>(&'a self, resolved: Option<&'a str>) -> Option<&'a str> {
        resolved.or(self.compatibility_message.as_deref())
    }

    #[must_use]
    pub fn with_related(
        mut self,
        related: impl IntoIterator<Item = DiagnosticRelatedPresentation>,
    ) -> Self {
        self.related.extend(related);
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<Option<DiagnosticPresentation>>) -> Self {
        self.help = help.into();
        self
    }

    #[must_use]
    pub fn with_explanation(
        mut self,
        explanation: impl Into<Option<DiagnosticExplanationPresentation>>,
    ) -> Self {
        self.explanation = explanation.into();
        self
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticRecordWire {
    version: u16,
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    #[serde(deserialize_with = "deserialize_strict_source_span")]
    span: SourceSpan,
    presentation: DiagnosticPresentation,
    related: Vec<DiagnosticRelatedPresentation>,
    help: Option<DiagnosticPresentation>,
    explanation: Option<DiagnosticExplanationPresentation>,
    compatibility_message: Option<String>,
}

impl<'de> Deserialize<'de> for DiagnosticRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticRecordWire::deserialize(deserializer)?;
        if wire.version != DIAGNOSTIC_RECORD_VERSION {
            return Err(serde::de::Error::custom(format_args!(
                "unsupported diagnostic record version {}; expected {}",
                wire.version, DIAGNOSTIC_RECORD_VERSION
            )));
        }

        Ok(Self {
            version: wire.version,
            code: wire.code,
            severity: wire.severity,
            span: wire.span,
            presentation: wire.presentation,
            related: wire.related,
            help: wire.help,
            explanation: wire.explanation,
            compatibility_message: wire.compatibility_message,
        })
    }
}

/// Deserialize a source span with the closed shape required by durable
/// diagnostic records. The public [`SourceSpan`] type remains permissive for
/// unrelated source-level formats.
pub(crate) fn deserialize_strict_source_span<'de, D>(
    deserializer: D,
) -> Result<SourceSpan, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SourcePositionWire {
        line: u32,
        column: u32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SourceSpanWire {
        file: String,
        start: SourcePositionWire,
        end: Option<SourcePositionWire>,
    }

    fn position<E>(wire: SourcePositionWire) -> Result<SourcePosition, E>
    where
        E: serde::de::Error,
    {
        SourcePosition::new(wire.line, wire.column).map_err(E::custom)
    }

    let wire = SourceSpanWire::deserialize(deserializer)?;
    let start = position::<D::Error>(wire.start)?;
    let end = wire.end.map(position::<D::Error>).transpose()?;
    Ok(SourceSpan::new(wire.file, start, end))
}
