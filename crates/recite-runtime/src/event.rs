use recite_core::{ChoiceId, LineId, MetadataEntry, SpeakerId};

/// Structured output emitted by runtime traversal.
#[derive(Clone, Debug, PartialEq)]
pub enum DialogueEvent {
    Line(DialogueLine),
    Prompt {
        line: Option<DialogueLine>,
        choices: Vec<DialogueChoice>,
    },
    End,
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
