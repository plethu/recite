use serde::Serialize;

use crate::SourceSpan;
use crate::diagnostic_presentation_record::DiagnosticPresentation;

/// A related source span paired with its own structured presentation.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub struct DiagnosticRelatedPresentation {
    pub span: SourceSpan,
    pub presentation: DiagnosticPresentation,
}

impl DiagnosticRelatedPresentation {
    #[must_use]
    pub fn new(span: SourceSpan, presentation: DiagnosticPresentation) -> Self {
        Self { span, presentation }
    }
}

/// Structured, localisable explanation and guidance hooks.
///
/// `remediation` is an ordered set of guidance presentations. Keeping each
/// item as a presentation, instead of a rendered string, leaves inventory
/// authors free to localise and evolve the explanation independently.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub struct DiagnosticExplanationPresentation {
    pub meaning: DiagnosticPresentation,
    pub common_causes: Vec<DiagnosticPresentation>,
    pub remediation: Vec<DiagnosticPresentation>,
}

impl DiagnosticExplanationPresentation {
    #[must_use]
    pub fn new(meaning: DiagnosticPresentation) -> Self {
        Self {
            meaning,
            common_causes: Vec::new(),
            remediation: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_common_causes(
        mut self,
        causes: impl IntoIterator<Item = DiagnosticPresentation>,
    ) -> Self {
        self.common_causes.extend(causes);
        self
    }

    #[must_use]
    pub fn with_remediation(
        mut self,
        guidance: impl IntoIterator<Item = DiagnosticPresentation>,
    ) -> Self {
        self.remediation.extend(guidance);
        self
    }
}
