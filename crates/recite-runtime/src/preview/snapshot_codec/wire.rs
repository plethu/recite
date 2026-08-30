use recite_core::SourceSpan;
use serde::{Deserialize, Serialize};

use crate::DialogueSessionSnapshot;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct SnapshotWire {
    pub(super) version: u16,
    pub(super) session: DialogueSessionSnapshot,
    pub(super) initial_block: Option<String>,
    pub(super) locale: Option<String>,
    pub(super) variant: Option<String>,
    pub(super) next_condition_id: u64,
    pub(super) state: StateWire,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct StateWire {
    pub(super) asset_id: String,
    pub(super) block: Option<String>,
    pub(super) locale: Option<String>,
    pub(super) selected_choices: Vec<String>,
    pub(super) deferred_effects: Vec<EffectWire>,
    pub(super) restart_required: Option<RequirementWire>,
    pub(super) status: StatusWire,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) enum StatusWire {
    Ready,
    WaitingForChoice { prompt: Box<PromptWire> },
    WaitingForEffect { effect: EffectWire },
    Ended,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct RequirementWire {
    pub(super) active_asset: String,
    pub(super) replacement_asset: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PromptWire {
    pub(super) block: String,
    pub(super) line: Option<String>,
    pub(super) choices: Vec<String>,
    pub(super) line_projection: Option<LineWire>,
    pub(super) choice_projection: Vec<ChoiceWire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct LineWire {
    pub(super) id: String,
    pub(super) source_text: String,
    pub(super) text: String,
    pub(super) speaker: Option<String>,
    pub(super) metadata: Vec<MetadataWire>,
    pub(super) plural: Option<PluralWire>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct MetadataWire {
    pub(super) key: String,
    pub(super) value: ValueWire,
    pub(super) source_span: Option<SourceSpan>,
    pub(super) key_span: Option<SourceSpan>,
    pub(super) value_span: Option<SourceSpan>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) enum ValueWire {
    Scalar(ArgumentWire),
    Array(Vec<ArgumentWire>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PluralWire {
    pub(super) singular_source_text: String,
    pub(super) plural_source_text: String,
    pub(super) count: i64,
    pub(super) selected_arm: usize,
    pub(super) resolution: PluralResolutionWire,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PluralResolutionWire {
    pub(super) attempts: Vec<PluralAttemptWire>,
    pub(super) matched_locale: Option<String>,
    pub(super) matched_context: Option<String>,
    pub(super) matched_key: Option<String>,
    pub(super) matched_arm: Option<usize>,
    pub(super) source_fallback_arm: Option<usize>,
    pub(super) outcome: PluralOutcomeWire,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PluralAttemptWire {
    pub(super) locale: String,
    pub(super) context: String,
    pub(super) key: String,
    pub(super) selected_arm: Option<usize>,
    pub(super) outcome: PluralOutcomeWire,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum PluralOutcomeWire {
    MissingPluralForms,
    MissingEntry,
    MissingTranslation,
    Matched,
    Translated,
    EnglishSourceFallback,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ChoiceWire {
    pub(super) id: String,
    pub(super) source_text: String,
    pub(super) text: String,
    pub(super) availability: crate::DialogueChoiceAvailabilitySnapshot,
    pub(super) metadata: Vec<MetadataWire>,
    pub(super) echo: EchoWire,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) enum EchoWire {
    None,
    SelectedText,
    ExplicitLine(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct EffectWire {
    pub(super) id: String,
    pub(super) mode: DialogueEffectModeWire,
    pub(super) function: String,
    pub(super) args: Vec<ArgumentWire>,
    pub(super) source_span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(super) enum DialogueEffectModeWire {
    Deferred,
    Immediate,
    Blocking,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) enum ArgumentWire {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}
