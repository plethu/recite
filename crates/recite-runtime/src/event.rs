use recite_core::{
    AvailabilityReasonId, ChoiceId, EffectId, LineId, MetadataEntry, SourceSpan, SpeakerId,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceAvailabilityReason {
    pub id: AvailabilityReasonId,
    pub source_text: String,
    pub origin: Option<ChoiceAvailabilityReasonOrigin>,
    pub args: Vec<ChoiceAvailabilityReasonArg>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChoiceAvailabilityReasonOrigin {
    ConditionCall { function: String, args: Vec<String> },
    RequirementExpression { source_text: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceAvailabilityReasonArg {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
