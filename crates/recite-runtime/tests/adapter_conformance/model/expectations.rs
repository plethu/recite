use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StepExpectation {
    pub(crate) status: StepStatus,
    pub(crate) event_kind: Option<EventKind>,
    pub(crate) line_text: Option<String>,
    pub(crate) line_plural: Option<PluralLineExpectation>,
    pub(crate) prompt_choice_ids: Option<Vec<String>>,
    pub(crate) prompt_unavailable_choice_ids: Option<Vec<String>>,
    pub(crate) prompt_choice_availability: Option<Vec<ChoiceAvailabilityExpectation>>,
    pub(crate) effect_function: Option<String>,
    pub(crate) effect_mode: Option<EffectMode>,
    pub(crate) deferred_effect_functions: Option<Vec<String>>,
    pub(crate) pending_effect_slot: Option<String>,
    pub(crate) error_category: Option<String>,
    pub(crate) allowed_error_categories: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluralLineExpectation {
    pub(crate) singular_source_text: String,
    pub(crate) plural_source_text: String,
    pub(crate) count: i64,
    pub(crate) selected_arm: usize,
    pub(crate) resolution: PluralResolutionExpectation,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluralResolutionExpectation {
    pub(crate) attempts: Vec<PluralAttemptExpectation>,
    pub(crate) matched_locale: Option<String>,
    pub(crate) matched_context: Option<String>,
    pub(crate) matched_key: Option<String>,
    pub(crate) matched_arm: Option<usize>,
    pub(crate) source_fallback_arm: Option<usize>,
    pub(crate) outcome: PluralResolutionOutcomeExpectation,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluralAttemptExpectation {
    pub(crate) locale: String,
    pub(crate) context: String,
    pub(crate) key: String,
    pub(crate) selected_arm: Option<usize>,
    pub(crate) outcome: PluralAttemptOutcomeExpectation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PluralResolutionOutcomeExpectation {
    Translated,
    EnglishSourceFallback,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PluralAttemptOutcomeExpectation {
    MissingPluralForms,
    MissingEntry,
    MissingTranslation,
    Matched,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChoiceAvailabilityExpectation {
    pub(crate) choice_id: String,
    pub(crate) is_available: bool,
    pub(crate) primary_reason: Option<AvailabilityReasonExpectation>,
    pub(crate) reason_tree: Option<AvailabilityReasonTreeExpectation>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AvailabilityReasonExpectation {
    pub(crate) id: String,
    pub(crate) source_text: String,
    pub(crate) text: String,
    pub(crate) origin: Option<AvailabilityReasonOriginExpectation>,
    pub(crate) args: Vec<AvailabilityReasonArgExpectation>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AvailabilityReasonOriginExpectation {
    ConditionCall {
        function: String,
        args: Vec<AvailabilityReasonValueExpectation>,
    },
    RequirementExpression {
        source_text: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AvailabilityReasonArgExpectation {
    pub(crate) name: String,
    pub(crate) value: AvailabilityReasonValueExpectation,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct AvailabilityReasonValueExpectation {
    pub(crate) kind: AvailabilityReasonValueKind,
    pub(crate) value: Value,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AvailabilityReasonValueKind {
    Identifier,
    String,
    Integer,
    Float,
    Boolean,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AvailabilityReasonTreeExpectation {
    All {
        children: Vec<AvailabilityReasonTreeExpectation>,
    },
    Any {
        children: Vec<AvailabilityReasonTreeExpectation>,
    },
    Reason {
        reason: AvailabilityReasonExpectation,
    },
    RequirementSourceText {
        source_text: String,
    },
}

impl StepExpectation {
    pub(crate) fn expected_error(&self) -> Option<super::core::ExpectedError> {
        if let Some(error_category) = &self.error_category {
            return Some(super::core::ExpectedError::Single {
                error_category: error_category.clone(),
            });
        }
        self.allowed_error_categories
            .as_ref()
            .map(
                |allowed_error_categories| super::core::ExpectedError::Allowed {
                    allowed_error_categories: allowed_error_categories.clone(),
                },
            )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StepStatus {
    Ok,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventKind {
    Line,
    Prompt,
    Effect,
    End,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EffectMode {
    Deferred,
    Immediate,
    Blocking,
}
