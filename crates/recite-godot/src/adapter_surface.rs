use std::path::Path;

use recite_core::compiled::CompiledDialogue;
use recite_runtime::{ConditionArgument, ConditionExpectedType, ConditionQuery};

use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};

#[derive(Clone, Debug)]
pub struct ReciteDialogueAsset {
    dialogue: recite_adapter::LoadedDialogue,
}

impl ReciteDialogueAsset {
    pub fn load_from_bytes(bytes: &[u8]) -> AdapterResult<Self> {
        Ok(Self {
            dialogue: recite_adapter::LoadedDialogue::from_bytes(bytes)?,
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
        self.dialogue.dialogue()
    }

    #[must_use]
    pub fn asset_id(&self) -> &str {
        self.dialogue.asset_id()
    }

    pub fn content_identity(&self) -> AdapterResult<String> {
        self.dialogue.content_identity()
    }

    pub(crate) fn loaded(&self) -> &recite_adapter::LoadedDialogue {
        &self.dialogue
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

pub use recite_runtime::DialogueEvent as ReciteOutput;
