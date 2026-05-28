use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum GenerateMode {
    Project,
    Summary,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FixtureConfigSet {
    #[serde(default)]
    profiles: BTreeMap<String, FixtureProfile>,
}

impl FixtureConfigSet {
    pub fn load_path(path: &Path) -> Result<Self, FixtureError> {
        let source = fs::read_to_string(path).map_err(FixtureError::Io)?;
        toml::from_str(&source).map_err(FixtureError::Toml)
    }

    pub fn profile(&self, name: &str) -> Result<&FixtureProfile, FixtureError> {
        self.profiles
            .get(name)
            .ok_or_else(|| FixtureError::UnknownProfile(name.to_owned()))
    }

    pub fn profiles(&self) -> impl Iterator<Item = (&str, &FixtureProfile)> {
        self.profiles
            .iter()
            .map(|(name, profile)| (name.as_str(), profile))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureProfile {
    pub name: String,
    pub seed: u64,
    pub blocks: u32,
    pub lines: u32,
    pub choices: u32,
    pub localisable_entries: u32,
    pub generated_words: u32,
    pub shards: u32,
}

impl FixtureProfile {
    pub(crate) fn validate(&self) -> Result<(), FixtureError> {
        if self.blocks == 0 {
            return Err(FixtureError::InvalidProfile("blocks must be positive"));
        }
        if self.lines < self.blocks {
            return Err(FixtureError::InvalidProfile(
                "lines must be at least blocks",
            ));
        }
        if self.choices < self.blocks {
            return Err(FixtureError::InvalidProfile(
                "choices must be at least blocks",
            ));
        }
        if self.localisable_entries != self.lines + self.choices {
            return Err(FixtureError::InvalidProfile(
                "localisable_entries must equal lines + choices",
            ));
        }
        if self.shards == 0 || self.shards > self.blocks {
            return Err(FixtureError::InvalidProfile(
                "shards must be positive and no greater than blocks",
            ));
        }
        Ok(())
    }

    pub(crate) fn words_per_entry(&self) -> u32 {
        self.generated_words.div_ceil(self.localisable_entries)
    }
}

#[derive(Debug)]
pub enum FixtureError {
    Io(io::Error),
    Toml(toml::de::Error),
    Json(serde_json::Error),
    UnknownProfile(String),
    InvalidProfile(&'static str),
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Toml(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::UnknownProfile(profile) => write!(formatter, "unknown profile `{profile}`"),
            Self::InvalidProfile(message) => write!(formatter, "invalid profile: {message}"),
        }
    }
}

impl std::error::Error for FixtureError {}
