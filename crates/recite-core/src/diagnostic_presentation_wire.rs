use serde::{Deserialize, Deserializer};

use crate::SourceSpan;
use crate::diagnostic_presentation_guidance::{
    DiagnosticExplanationPresentation, DiagnosticRelatedPresentation,
};
use crate::diagnostic_presentation_record::DiagnosticPresentation;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticRelatedPresentationWire {
    #[serde(deserialize_with = "crate::diagnostic_record::deserialize_strict_source_span")]
    span: SourceSpan,
    presentation: DiagnosticPresentation,
}

impl<'de> Deserialize<'de> for DiagnosticRelatedPresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticRelatedPresentationWire::deserialize(deserializer)?;
        Ok(Self::new(wire.span, wire.presentation))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticExplanationPresentationWire {
    meaning: DiagnosticPresentation,
    common_causes: Vec<DiagnosticPresentation>,
    remediation: Vec<DiagnosticPresentation>,
}

impl<'de> Deserialize<'de> for DiagnosticExplanationPresentation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DiagnosticExplanationPresentationWire::deserialize(deserializer)?;
        Ok(Self {
            meaning: wire.meaning,
            common_causes: wire.common_causes,
            remediation: wire.remediation,
        })
    }
}
