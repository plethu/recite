use recite_core::{MetadataEntry, ScalarValue, SpeakerId, Value};

use super::super::model::PreviewError;
use super::wire::{
    ArgumentWire, LineWire, MetadataWire, PluralAttemptWire, PluralOutcomeWire,
    PluralResolutionWire, PluralWire, ValueWire,
};
use crate::locale::{PluralResolutionAttempt, PluralResolutionOutcome};
use crate::{
    DialogueLine, DialoguePlural, DialoguePluralResolution, DialoguePluralResolutionOutcome,
};

impl LineWire {
    pub(super) fn from_line(line: &DialogueLine) -> Self {
        Self {
            id: line.id.to_string(),
            source_text: line.source_text.clone(),
            text: line.text.clone(),
            speaker: line.speaker.as_ref().map(ToString::to_string),
            metadata: line.metadata.iter().map(MetadataWire::from_entry).collect(),
            plural: line.plural.as_ref().map(PluralWire::from_plural),
        }
    }

    pub(super) fn into_line(self) -> Result<DialogueLine, PreviewError> {
        Ok(DialogueLine {
            id: recite_core::LineId::new(self.id).map_err(invalid)?,
            source_text: self.source_text,
            text: self.text,
            speaker: self
                .speaker
                .map(SpeakerId::new)
                .transpose()
                .map_err(invalid)?,
            metadata: self
                .metadata
                .into_iter()
                .map(MetadataWire::into_entry)
                .collect::<Result<Vec<_>, _>>()?,
            plural: self.plural.map(PluralWire::into_plural).transpose()?,
        })
    }
}

impl MetadataWire {
    pub(super) fn from_entry(entry: &MetadataEntry) -> Self {
        Self {
            key: entry.key.clone(),
            value: ValueWire::from_value(&entry.value),
            source_span: entry.source_span.clone(),
            key_span: entry.key_span.clone(),
            value_span: entry.value_span.clone(),
        }
    }

    pub(super) fn into_entry(self) -> Result<MetadataEntry, PreviewError> {
        Ok(MetadataEntry {
            key: self.key,
            value: self.value.into_value()?,
            source_span: self.source_span,
            key_span: self.key_span,
            value_span: self.value_span,
        })
    }
}

impl ValueWire {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Scalar(value) => Self::Scalar(ArgumentWire::from_scalar(value)),
            Value::Array(values) => {
                Self::Array(values.iter().map(ArgumentWire::from_scalar).collect())
            }
        }
    }

    fn into_value(self) -> Result<Value, PreviewError> {
        match self {
            Self::Scalar(value) => Ok(Value::Scalar(value.into_scalar()?)),
            Self::Array(values) => Ok(Value::Array(
                values
                    .into_iter()
                    .map(ArgumentWire::into_scalar)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        }
    }
}

impl ArgumentWire {
    pub(super) fn from_scalar(value: &ScalarValue) -> Self {
        match value {
            ScalarValue::String(value) => Self::String(value.clone()),
            ScalarValue::Integer(value) => Self::Integer(*value),
            ScalarValue::Float(value) => Self::Float(*value),
            ScalarValue::Boolean(value) => Self::Boolean(*value),
        }
    }

    pub(super) fn into_scalar(self) -> Result<ScalarValue, PreviewError> {
        Ok(match self {
            Self::String(value) => ScalarValue::String(value),
            Self::Integer(value) => ScalarValue::Integer(value),
            Self::Float(value) => ScalarValue::Float(value),
            Self::Boolean(value) => ScalarValue::Boolean(value),
            Self::Identifier(_) => return Err(invalid("identifier is not a metadata scalar")),
        })
    }
}

impl PluralWire {
    fn from_plural(plural: &DialoguePlural) -> Self {
        Self {
            singular_source_text: plural.singular_source_text.clone(),
            plural_source_text: plural.plural_source_text.clone(),
            count: plural.count,
            selected_arm: plural.selected_arm,
            resolution: PluralResolutionWire::from_resolution(&plural.resolution),
        }
    }

    fn into_plural(self) -> Result<DialoguePlural, PreviewError> {
        Ok(DialoguePlural {
            singular_source_text: self.singular_source_text,
            plural_source_text: self.plural_source_text,
            count: self.count,
            selected_arm: self.selected_arm,
            resolution: self.resolution.into_resolution()?,
        })
    }
}

impl PluralResolutionWire {
    fn from_resolution(resolution: &DialoguePluralResolution) -> Self {
        Self {
            attempts: resolution
                .attempts
                .iter()
                .map(PluralAttemptWire::from_attempt)
                .collect(),
            matched_locale: resolution.matched_locale.clone(),
            matched_context: resolution.matched_context.clone(),
            matched_key: resolution.matched_key.clone(),
            matched_arm: resolution.matched_arm,
            source_fallback_arm: resolution.source_fallback_arm,
            outcome: match resolution.outcome {
                DialoguePluralResolutionOutcome::Translated => PluralOutcomeWire::Translated,
                DialoguePluralResolutionOutcome::EnglishSourceFallback => {
                    PluralOutcomeWire::EnglishSourceFallback
                }
            },
        }
    }

    fn into_resolution(self) -> Result<DialoguePluralResolution, PreviewError> {
        Ok(DialoguePluralResolution {
            attempts: self
                .attempts
                .into_iter()
                .map(PluralAttemptWire::into_attempt)
                .collect::<Result<Vec<_>, _>>()?,
            matched_locale: self.matched_locale,
            matched_context: self.matched_context,
            matched_key: self.matched_key,
            matched_arm: self.matched_arm,
            source_fallback_arm: self.source_fallback_arm,
            outcome: match self.outcome {
                PluralOutcomeWire::Translated => DialoguePluralResolutionOutcome::Translated,
                PluralOutcomeWire::EnglishSourceFallback => {
                    DialoguePluralResolutionOutcome::EnglishSourceFallback
                }
                _ => return Err(invalid("invalid plural resolution outcome")),
            },
        })
    }
}

impl PluralAttemptWire {
    fn from_attempt(attempt: &PluralResolutionAttempt) -> Self {
        Self {
            locale: attempt.locale.clone(),
            context: attempt.context.clone(),
            key: attempt.key.clone(),
            selected_arm: attempt.selected_arm,
            outcome: match attempt.outcome {
                PluralResolutionOutcome::MissingPluralForms => {
                    PluralOutcomeWire::MissingPluralForms
                }
                PluralResolutionOutcome::MissingEntry => PluralOutcomeWire::MissingEntry,
                PluralResolutionOutcome::MissingTranslation => {
                    PluralOutcomeWire::MissingTranslation
                }
                PluralResolutionOutcome::Matched => PluralOutcomeWire::Matched,
            },
        }
    }

    fn into_attempt(self) -> Result<PluralResolutionAttempt, PreviewError> {
        Ok(PluralResolutionAttempt {
            locale: self.locale,
            context: self.context,
            key: self.key,
            selected_arm: self.selected_arm,
            outcome: match self.outcome {
                PluralOutcomeWire::MissingPluralForms => {
                    PluralResolutionOutcome::MissingPluralForms
                }
                PluralOutcomeWire::MissingEntry => PluralResolutionOutcome::MissingEntry,
                PluralOutcomeWire::MissingTranslation => {
                    PluralResolutionOutcome::MissingTranslation
                }
                PluralOutcomeWire::Matched => PluralResolutionOutcome::Matched,
                _ => return Err(invalid("invalid plural attempt outcome")),
            },
        })
    }
}

fn invalid(error: impl std::fmt::Display) -> PreviewError {
    PreviewError::SnapshotDecodeFailed {
        reason: error.to_string(),
    }
}
