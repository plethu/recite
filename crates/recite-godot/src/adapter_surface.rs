use std::path::Path;
use std::sync::Arc;

use recite_core::{CompiledDialogue, decode_compiled_dialogue_messagepack};
use recite_runtime::{
    ConditionArgument, ConditionExpectedType, ConditionQuery, DialogueChoice,
    DialogueEffectRequest, DialogueEvent, DialogueLine,
};

use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};

#[derive(Clone, Debug)]
pub struct ReciteDialogueAsset {
    dialogue: Arc<CompiledDialogue>,
}

impl ReciteDialogueAsset {
    pub fn load_from_bytes(bytes: &[u8]) -> AdapterResult<Self> {
        let dialogue = decode_compiled_dialogue_messagepack(bytes).map_err(AdapterError::from)?;
        Ok(Self {
            dialogue: Arc::new(dialogue),
        })
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> AdapterResult<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|error| {
            AdapterError::with_detail(
                AdapterErrorKind::AssetLoadOrDecode,
                format!("failed to read `{}`: {error}", path.display()),
            )
        })?;
        Self::load_from_bytes(&bytes)
    }

    #[must_use]
    pub fn dialogue(&self) -> &CompiledDialogue {
        &self.dialogue
    }

    #[must_use]
    pub fn shared_dialogue(&self) -> Arc<CompiledDialogue> {
        Arc::clone(&self.dialogue)
    }

    #[must_use]
    pub fn asset_id(&self) -> &str {
        self.dialogue.header.asset_id.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConditionCall<'a> {
    pub(crate) query: ConditionQuery<'a>,
}

impl<'a> ConditionCall<'a> {
    #[must_use]
    pub fn function(self) -> &'a str {
        self.query.function()
    }

    #[must_use]
    pub fn expected_type(self) -> ConditionExpectedType {
        self.query.expected_type()
    }

    pub fn arguments(self) -> impl Iterator<Item = AdapterValue> + 'a {
        self.query.arguments().into_iter().map(AdapterValue::from)
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum AdapterValue {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<ConditionArgument<'_>> for AdapterValue {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value.to_owned()),
            ConditionArgument::String(value) => Self::String(value.to_owned()),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ReciteOutput {
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

impl From<DialogueEvent> for ReciteOutput {
    fn from(event: DialogueEvent) -> Self {
        match event {
            DialogueEvent::Line(line) => Self::Line(line),
            DialogueEvent::Prompt { line, choices } => Self::Prompt { line, choices },
            DialogueEvent::Effect(effect) => Self::Effect(effect),
            DialogueEvent::End { deferred_effects } => Self::End { deferred_effects },
        }
    }
}
