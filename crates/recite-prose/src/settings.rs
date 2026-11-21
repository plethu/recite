//! Resolution of a project's `[prose]` table into checker settings.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use recite_core::project::ProjectProse;

use crate::ProseSetupError;

/// A prose language with a bundled dictionary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[non_exhaustive]
pub enum ProseLanguage {
    #[default]
    EnglishUs,
    EnglishGb,
    EnglishAu,
    EnglishCa,
    EnglishIn,
}

impl ProseLanguage {
    /// Every supported language, in tag order.
    pub const ALL: [Self; 5] = [
        Self::EnglishAu,
        Self::EnglishCa,
        Self::EnglishGb,
        Self::EnglishIn,
        Self::EnglishUs,
    ];

    /// Parse a BCP 47 tag, ignoring ASCII case; `en` means `en-US`.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        if tag.eq_ignore_ascii_case("en") {
            return Some(Self::EnglishUs);
        }
        Self::ALL
            .into_iter()
            .find(|language| language.tag().eq_ignore_ascii_case(tag))
    }

    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::EnglishUs => "en-US",
            Self::EnglishGb => "en-GB",
            Self::EnglishAu => "en-AU",
            Self::EnglishCa => "en-CA",
            Self::EnglishIn => "en-IN",
        }
    }
}

/// Validated `[prose]` settings for one project.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct ProseSettings {
    enabled: bool,
    language: ProseLanguage,
    dictionaries: Vec<PathBuf>,
    ignore_rules: BTreeSet<String>,
    ignore_speakers: BTreeSet<String>,
    ignore_ids: Vec<String>,
    metadata_keys: BTreeSet<String>,
}

impl ProseSettings {
    /// Resolve the manifest table against the project root. Dictionary paths
    /// must stay inside the project; missing files are treated as empty so an
    /// editor can create the first one when a word is added.
    pub fn from_project(
        project_root: &Path,
        prose: &ProjectProse,
    ) -> Result<Self, ProseSetupError> {
        let language = match prose.language.as_deref() {
            None => ProseLanguage::default(),
            Some(tag) => ProseLanguage::from_tag(tag)
                .ok_or_else(|| ProseSetupError::UnsupportedLanguage(tag.to_owned()))?,
        };
        let dictionaries = prose
            .dictionaries
            .iter()
            .map(|relative| project_path(project_root, relative))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            enabled: prose.enabled,
            language,
            dictionaries,
            ignore_rules: prose.ignore_rules.iter().cloned().collect(),
            ignore_speakers: prose.ignore_speakers.iter().cloned().collect(),
            ignore_ids: prose.ignore_ids.clone(),
            metadata_keys: prose.metadata_keys.iter().cloned().collect(),
        })
    }

    /// Whether editors should publish findings while authors type.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn language(&self) -> ProseLanguage {
        self.language
    }

    /// Absolute dictionary paths in declared order.
    #[must_use]
    pub fn dictionaries(&self) -> &[PathBuf] {
        &self.dictionaries
    }

    /// The dictionary that receives words added from an editor.
    #[must_use]
    pub fn dictionary_for_additions(&self) -> Option<&Path> {
        self.dictionaries.first().map(PathBuf::as_path)
    }

    #[must_use]
    pub fn ignores_rule(&self, rule: &str) -> bool {
        self.ignore_rules.contains(rule)
    }

    #[cfg_attr(
        not(feature = "harper"),
        allow(dead_code, reason = "used by the Harper engine")
    )]
    pub(crate) fn ignored_rules(&self) -> impl Iterator<Item = &str> {
        self.ignore_rules.iter().map(String::as_str)
    }

    /// Whether a region owned by this speaker and stable ID is skipped. An
    /// `ignore_ids` entry ending in `*` matches IDs with that prefix.
    #[must_use]
    pub fn ignores_owner(&self, speaker: Option<&str>, stable_id: Option<&str>) -> bool {
        speaker.is_some_and(|speaker| self.ignore_speakers.contains(speaker))
            || stable_id.is_some_and(|id| {
                self.ignore_ids
                    .iter()
                    .any(|pattern| match pattern.strip_suffix('*') {
                        Some(prefix) => id.starts_with(prefix),
                        None => id == pattern,
                    })
            })
    }

    /// Whether quoted values of this metadata key are checked as prose.
    #[must_use]
    pub fn checks_metadata_key(&self, key: &str) -> bool {
        self.metadata_keys.contains(key)
    }
}

fn project_path(project_root: &Path, relative: &str) -> Result<PathBuf, ProseSetupError> {
    let path = Path::new(relative);
    let escapes = path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if relative.is_empty() || escapes {
        return Err(ProseSetupError::Dictionary {
            path: relative.to_owned(),
            detail: "the path must be relative to the project root and stay inside it".to_owned(),
        });
    }
    Ok(project_root.join(path))
}
