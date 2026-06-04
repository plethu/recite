#[derive(Serialize)]
pub(crate) struct TraceDocument {
    asset_id: String,
    block: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialogue_locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialogue_locale_fallbacks: Option<Vec<String>>,
    events: Vec<TraceEvent>,
    final_deferred_effects: Vec<TraceEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: Option<TraceMetrics>,
}

impl TraceDocument {
    pub(in crate::runtime_fixture) fn new(
        asset_id: String,
        block: String,
        dialogue_locale: Option<String>,
        dialogue_locale_fallbacks: Option<Vec<String>>,
        events: Vec<TraceEvent>,
        final_deferred_effects: Vec<TraceEffect>,
        metrics: Option<TraceMetrics>,
    ) -> Self {
        Self {
            asset_id,
            block,
            dialogue_locale,
            dialogue_locale_fallbacks,
            events,
            final_deferred_effects,
            metrics,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceMetrics {
    pub(in crate::runtime_fixture) event_count: usize,
    pub(in crate::runtime_fixture) line_count: usize,
    pub(in crate::runtime_fixture) prompt_count: usize,
    pub(in crate::runtime_fixture) choice_count: usize,
    pub(in crate::runtime_fixture) condition_evaluation_count: usize,
    pub(in crate::runtime_fixture) effect_count: TraceEffectCounts,
    pub(in crate::runtime_fixture) localization_lookup_count: usize,
    pub(in crate::runtime_fixture) elapsed_traversal_time_ns: u128,
    pub(in crate::runtime_fixture) max_serialized_session_size_bytes: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(in crate::runtime_fixture) struct TraceEffectCounts {
    pub(in crate::runtime_fixture) deferred: usize,
    pub(in crate::runtime_fixture) immediate: usize,
    pub(in crate::runtime_fixture) blocking: usize,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceEvent {
    Condition {
        condition: TraceCondition,
    },
    Line {
        line: TraceLine,
    },
    Prompt {
        prompt: TracePrompt,
    },
    ChoiceSelected {
        prompt: TracePromptIdentity,
        choice: String,
    },
    Effect {
        effect: TraceEffect,
    },
    Acknowledgement {
        effect_id: String,
        result: &'static str,
    },
    End {
        deferred_effects: Vec<TraceEffect>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceCondition {
    pub(in crate::runtime_fixture) query: String,
    pub(in crate::runtime_fixture) function: String,
    pub(in crate::runtime_fixture) arguments: Vec<TraceScalar>,
    pub(in crate::runtime_fixture) result: TraceConditionValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(in crate::runtime_fixture) enum TraceConditionValue {
    Bool(bool),
    EnumVariant { r#enum: String },
}

impl std::fmt::Display for TraceConditionValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::EnumVariant { r#enum } => write!(formatter, "enum {enum}"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TracePrompt {
    pub(in crate::runtime_fixture) identity: TracePromptIdentity,
    pub(in crate::runtime_fixture) line: Option<TraceLine>,
    pub(in crate::runtime_fixture) choices: Vec<TraceChoice>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TracePromptIdentity {
    pub(in crate::runtime_fixture) block: String,
    pub(in crate::runtime_fixture) line: Option<String>,
    pub(in crate::runtime_fixture) fixture_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceLine {
    pub(in crate::runtime_fixture) id: String,
    pub(in crate::runtime_fixture) source_text: String,
    pub(in crate::runtime_fixture) text: String,
    pub(in crate::runtime_fixture) speaker: Option<String>,
    pub(in crate::runtime_fixture) metadata: Vec<TraceMetadata>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceChoice {
    pub(in crate::runtime_fixture) id: String,
    pub(in crate::runtime_fixture) source_text: String,
    pub(in crate::runtime_fixture) text: String,
    pub(in crate::runtime_fixture) metadata: Vec<TraceMetadata>,
    pub(in crate::runtime_fixture) is_available: bool,
    pub(in crate::runtime_fixture) availability: TraceChoiceAvailability,
    pub(in crate::runtime_fixture) unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceChoiceAvailability {
    pub(in crate::runtime_fixture) is_available: bool,
    pub(in crate::runtime_fixture) primary_reason: Option<TraceChoiceAvailabilityReason>,
    pub(in crate::runtime_fixture) reason_tree: Option<TraceChoiceAvailabilityReasonTree>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceChoiceAvailabilityReason {
    pub(in crate::runtime_fixture) id: String,
    pub(in crate::runtime_fixture) source_text: String,
    pub(in crate::runtime_fixture) text: String,
    pub(in crate::runtime_fixture) origin: Option<TraceChoiceAvailabilityReasonOrigin>,
    pub(in crate::runtime_fixture) args: Vec<TraceChoiceAvailabilityReasonArg>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceChoiceAvailabilityReasonOrigin {
    ConditionCall {
        function: String,
        args: Vec<TraceChoiceAvailabilityReasonValue>,
    },
    RequirementExpression {
        source_text: String,
    },
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceChoiceAvailabilityReasonArg {
    pub(in crate::runtime_fixture) name: String,
    pub(in crate::runtime_fixture) value: TraceChoiceAvailabilityReasonValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceChoiceAvailabilityReasonValue {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceChoiceAvailabilityReasonTree {
    All(Vec<TraceChoiceAvailabilityReasonTree>),
    Any(Vec<TraceChoiceAvailabilityReasonTree>),
    Reason(TraceChoiceAvailabilityReason),
    RequirementSourceText(String),
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceMetadata {
    pub(in crate::runtime_fixture) key: String,
    pub(in crate::runtime_fixture) value: TraceValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceValue {
    Scalar(TraceScalar),
    Array(Vec<TraceScalar>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub(in crate::runtime_fixture) enum TraceScalar {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceEffect {
    pub(in crate::runtime_fixture) id: String,
    pub(in crate::runtime_fixture) mode: &'static str,
    pub(in crate::runtime_fixture) function: String,
    pub(in crate::runtime_fixture) args: Vec<TraceScalar>,
    pub(in crate::runtime_fixture) source_span: TraceSourceSpan,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::runtime_fixture) struct TraceSourceSpan {
    pub(in crate::runtime_fixture) file: String,
    pub(in crate::runtime_fixture) start_line: u32,
    pub(in crate::runtime_fixture) start_column: u32,
    pub(in crate::runtime_fixture) end_line: Option<u32>,
    pub(in crate::runtime_fixture) end_column: Option<u32>,
}
use serde::Serialize;
