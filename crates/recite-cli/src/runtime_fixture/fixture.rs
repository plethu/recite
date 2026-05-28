use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use recite_core::{CompiledDialogue, decode_compiled_dialogue_messagepack};
use serde::Deserialize;

use crate::error::CliError;

pub(crate) fn load_compiled_asset(path: &Path) -> Result<CompiledDialogue, CliError> {
    let bytes = fs::read(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    decode_compiled_dialogue_messagepack(&bytes).map_err(|source| CliError::DecodeAsset {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn load_runtime_fixture(path: &Path) -> Result<RuntimeFixture, CliError> {
    let source = fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    toml::from_str(&source).map_err(|source| CliError::FixtureToml {
        path: path.to_owned(),
        source,
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeFixture {
    #[serde(default)]
    pub(super) conditions: BTreeMap<String, FixtureConditionValue>,
    #[serde(default)]
    pub(super) choices: BTreeMap<String, FixtureChoice>,
    #[serde(default)]
    pub(super) effects: FixtureEffects,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum FixtureConditionValue {
    Bool(bool),
    Enum { r#enum: String },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum FixtureChoice {
    Id(String),
    Index(usize),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FixtureEffects {
    #[serde(default)]
    pub(super) auto_ack_blocking: bool,
}
