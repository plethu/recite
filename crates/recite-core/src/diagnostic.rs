use serde::{Deserialize, Serialize};

use crate::{
    DiagnosticCode, DiagnosticExplanationPresentation, DiagnosticPresentation,
    DiagnosticPresentationError, DiagnosticRecord, DiagnosticRecordError,
    DiagnosticRelatedPresentation, SourceSpan,
};

mod contract;
mod explanation;

pub use contract::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract, DiagnosticPresentationContractRegistryError,
    auxiliary_contract_for, config_contract_for, contract_for, contracts_for_code,
    migrated_diagnostic_auxiliary_presentation_contracts,
    migrated_diagnostic_presentation_contracts, presentation_for,
    validate_auxiliary_diagnostic_presentation_contracts,
    validate_diagnostic_presentation_contracts,
    validate_migrated_diagnostic_presentation_contracts,
};
pub use explanation::{
    DiagnosticExplanation, default_presentation_id_for_code, explain_diagnostic_code,
    known_diagnostic_explanations, suggest_diagnostic_code,
};

/// Stable diagnostic severity shared by compiler, CLI, and LSP surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
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
    /// Explicitly supplied deterministic en-US compatibility fallback for
    /// [`Self::presentation`].
    ///
    /// New producers should treat the structured presentation as authoritative
    /// and populate this field only for compatibility with existing clients.
    pub message: String,
    pub span: SourceSpan,
    pub related: Vec<RelatedSpan>,
    pub help: Option<String>,
    /// Locale-neutral primary presentation. This remains optional while the
    /// existing diagnostic constructor families migrate to the structured
    /// contract.
    pub presentation: Option<DiagnosticPresentation>,
    /// Structured related presentations in source order.
    pub related_presentations: Vec<DiagnosticRelatedPresentation>,
    /// Locale-neutral help presentation.
    pub help_presentation: Option<DiagnosticPresentation>,
    /// Structured explanation and guidance hooks for the diagnostic inventory.
    pub explanation_presentation: Option<DiagnosticExplanationPresentation>,
}

impl Diagnostic {
    /// Construct a diagnostic without checking that `code` has a registered
    /// structured presentation contract. Extension producers may use this
    /// unchecked constructor when they own a separate presentation boundary;
    /// first-party producers should prefer [`Self::error_from_contract`].
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
            presentation: None,
            related_presentations: Vec::new(),
            help_presentation: None,
            explanation_presentation: None,
        }
    }

    /// Construct an error from a first-party contract and validate its exact
    /// presentation arguments before returning it.
    pub fn error_from_contract<I, K>(
        contract: &DiagnosticPresentationContract,
        message: impl Into<String>,
        span: SourceSpan,
        arguments: I,
    ) -> Result<Self, DiagnosticPresentationError>
    where
        I: IntoIterator<Item = (K, crate::DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        Self::from_contract(
            DiagnosticSeverity::Error,
            contract,
            message,
            span,
            arguments,
        )
    }

    /// Construct a diagnostic at any severity from a first-party contract and
    /// validate its exact presentation arguments before returning it.
    pub fn from_contract<I, K>(
        severity: DiagnosticSeverity,
        contract: &DiagnosticPresentationContract,
        message: impl Into<String>,
        span: SourceSpan,
        arguments: I,
    ) -> Result<Self, DiagnosticPresentationError>
    where
        I: IntoIterator<Item = (K, crate::DiagnosticArgumentValue)>,
        K: Into<String>,
    {
        let presentation = contract.presentation(arguments)?;
        Ok(Self::new(contract.code().clone(), severity, message, span)
            .with_presentation(presentation))
    }

    /// Construct an unchecked error diagnostic. The code is not resolved
    /// against the central presentation registry; first-party producers
    /// should prefer [`Self::error_from_contract`].
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

    /// Attach a locale-neutral primary presentation without checking its ID
    /// against `self.code`. Extension producers may use this unchecked
    /// builder; first-party producers should prefer [`Self::error_from_contract`].
    #[must_use]
    pub fn with_presentation(mut self, presentation: DiagnosticPresentation) -> Self {
        self.presentation = Some(presentation);
        self
    }

    /// Append structured related presentations while preserving source order.
    #[must_use]
    pub fn with_related_presentations(
        mut self,
        related: impl IntoIterator<Item = DiagnosticRelatedPresentation>,
    ) -> Self {
        self.related_presentations.extend(related);
        self
    }

    /// Attach a locale-neutral help presentation.
    #[must_use]
    pub fn with_help_presentation(mut self, help: DiagnosticPresentation) -> Self {
        self.help_presentation = Some(help);
        self
    }

    /// Attach structured explanation and guidance presentations.
    #[must_use]
    pub fn with_explanation_presentation(
        mut self,
        explanation: DiagnosticExplanationPresentation,
    ) -> Self {
        self.explanation_presentation = Some(explanation);
        self
    }

    /// Convert this diagnostic to its structured record without discarding
    /// legacy context.
    ///
    /// Legacy diagnostics return an error until their producer is migrated. A
    /// producer that mixes legacy related/help fields with structured fields
    /// also returns an error carrying the original ordered context rather than
    /// silently dropping it.
    #[must_use = "the structured conversion result reports incomplete or mixed legacy state"]
    pub fn record(&self) -> Result<DiagnosticRecord, DiagnosticRecordError> {
        if !self.related.is_empty() || self.help.is_some() {
            return Err(DiagnosticRecordError::LegacyContext {
                related: self.related.clone(),
                help: self.help.clone(),
            });
        }
        let Some(presentation) = self.presentation.clone() else {
            return Err(DiagnosticRecordError::MissingPresentation);
        };

        Ok(DiagnosticRecord::new(
            self.code.clone(),
            self.severity,
            self.span.clone(),
            presentation,
        )
        .with_related(self.related_presentations.clone())
        .with_help(self.help_presentation.clone())
        .with_explanation(self.explanation_presentation.clone())
        .with_compatibility_message(self.message.clone()))
    }
}
