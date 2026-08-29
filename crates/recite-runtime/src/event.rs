use recite_core::{
    AvailabilityReasonId, ChoiceId, EffectId, LineId, MetadataEntry, SourceSpan, SpeakerId,
};

use crate::locale::PluralResolutionAttempt;

/// Structured output emitted by runtime traversal.
#[derive(Clone, Debug, PartialEq)]
pub enum DialogueEvent {
    Line(DialogueLine),
    Prompt {
        line: Option<DialogueLine>,
        choices: Vec<DialogueChoice>,
    },
    Effect(DialogueEffectRequest),
    End {
        deferred_effects: Vec<DialogueEffectRequest>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialogueLine {
    pub id: LineId,
    pub source_text: String,
    pub text: String,
    pub speaker: Option<SpeakerId>,
    pub metadata: Vec<MetadataEntry>,
    pub plural: Option<DialoguePlural>,
}

/// Structured plural provenance attached to a delivered line.
///
/// This contains source forms and resolution metadata only. Localized
/// templates remain available through [`crate::DialogueTrace`].
#[derive(Clone, Debug, PartialEq)]
pub struct DialoguePlural {
    pub singular_source_text: String,
    pub plural_source_text: String,
    pub count: i64,
    pub selected_arm: usize,
    pub resolution: DialoguePluralResolution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialoguePluralResolution {
    pub attempts: Vec<PluralResolutionAttempt>,
    pub matched_locale: Option<String>,
    pub matched_context: Option<String>,
    pub matched_key: Option<String>,
    pub matched_arm: Option<usize>,
    pub source_fallback_arm: Option<usize>,
    pub outcome: DialoguePluralResolutionOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialoguePluralResolutionOutcome {
    Translated,
    EnglishSourceFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialogueChoice {
    pub id: ChoiceId,
    pub source_text: String,
    pub text: String,
    pub metadata: Vec<MetadataEntry>,
    pub availability: ChoiceAvailability,
    pub echo: ChoiceEchoMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAvailability {
    pub is_available: bool,
    pub primary_reason: Option<ChoiceAvailabilityReason>,
    pub reason_tree: Option<ChoiceAvailabilityReasonTree>,
}

impl ChoiceAvailability {
    #[must_use]
    pub fn available() -> Self {
        Self {
            is_available: true,
            primary_reason: None,
            reason_tree: None,
        }
    }

    #[must_use]
    pub fn unavailable(
        primary_reason: Option<ChoiceAvailabilityReason>,
        reason_tree: Option<ChoiceAvailabilityReasonTree>,
    ) -> Self {
        Self {
            is_available: false,
            primary_reason,
            reason_tree,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAvailabilityReason {
    pub id: AvailabilityReasonId,
    pub source_text: String,
    pub text: String,
    pub origin: Option<ChoiceAvailabilityReasonOrigin>,
    pub args: Vec<ChoiceAvailabilityReasonArg>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChoiceAvailabilityReasonOrigin {
    ConditionCall {
        function: String,
        args: Vec<ChoiceAvailabilityReasonValue>,
    },
    RequirementExpression {
        source_text: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAvailabilityReasonArg {
    pub name: String,
    pub value: ChoiceAvailabilityReasonValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChoiceAvailabilityReasonValue {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChoiceAvailabilityReasonTree {
    All(Vec<ChoiceAvailabilityReasonTree>),
    Any(Vec<ChoiceAvailabilityReasonTree>),
    Reason(ChoiceAvailabilityReason),
    RequirementSourceText(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChoiceEchoMode {
    None,
    SelectedText,
    ExplicitLine(LineId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialogueEffectRequest {
    pub id: EffectId,
    pub mode: DialogueEffectMode,
    pub function: String,
    pub args: Vec<DialogueEffectArgument>,
    pub source_span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogueEffectMode {
    Deferred,
    Immediate,
    Blocking,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogueEffectArgument {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectAck {
    Completed,
    Failed { reason: String },
}
