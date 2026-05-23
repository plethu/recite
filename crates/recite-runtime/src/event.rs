use recite_core::{ChoiceId, EffectId, LineId, MetadataEntry, SourceSpan, SpeakerId};

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
    pub is_available: bool,
    pub unavailable_reason: Option<String>,
    pub echo: ChoiceEchoMode,
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
