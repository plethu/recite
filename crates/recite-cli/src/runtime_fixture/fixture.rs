use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use recite_core::{CompiledDialogue, ScalarValue, decode_compiled_dialogue_messagepack};
use recite_runtime::{InterpolationValueProvider, LocaleError};
use serde::Deserialize;

use crate::dialogue_locale::DialoguePreviewConfig;
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
    let mut fixture: RuntimeFixture =
        toml::from_str(&source).map_err(|source| CliError::FixtureToml {
            path: path.to_owned(),
            source,
        })?;
    fixture.dialogue.resolve_paths(path);
    Ok(fixture)
}

pub(crate) fn dialogue_preview_from_fixture(
    fixture: &RuntimeFixture,
) -> Result<Option<DialoguePreviewConfig>, CliError> {
    fixture.dialogue.preview()
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FixtureDialogue {
    pub(super) locale: Option<String>,
    #[serde(default)]
    pub(super) catalogs: BTreeMap<String, Vec<PathBuf>>,
}

impl FixtureDialogue {
    fn resolve_paths(&mut self, fixture_path: &Path) {
        let Some(base) = fixture_path.parent() else {
            return;
        };
        for paths in self.catalogs.values_mut() {
            for path in paths {
                if !path.is_absolute() {
                    *path = base.join(&path);
                }
            }
        }
    }

    fn preview(&self) -> Result<Option<DialoguePreviewConfig>, CliError> {
        DialoguePreviewConfig::from_fixture(self.locale.as_deref(), &self.catalogs)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RuntimeFixture {
    #[serde(default)]
    pub(super) dialogue: FixtureDialogue,
    #[serde(default)]
    pub(super) conditions: BTreeMap<String, FixtureConditionValue>,
    #[serde(default)]
    pub(super) choices: BTreeMap<String, FixtureChoice>,
    #[serde(default)]
    pub(super) effects: FixtureEffects,
    #[serde(default)]
    pub(super) interpolation_values: FixtureInterpolationValues,
    #[serde(default, rename = "anchors")]
    _anchors: BTreeMap<String, String>,
}

impl RuntimeFixture {
    pub(super) fn interpolation_values(&self) -> &FixtureInterpolationValues {
        &self.interpolation_values
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(transparent)]
pub(super) struct FixtureInterpolationValues(BTreeMap<String, FixtureInterpolationValue>);

impl InterpolationValueProvider for FixtureInterpolationValues {
    fn lookup_value(&self, name: &str) -> Result<Option<ScalarValue>, LocaleError> {
        Ok(self.0.get(name).map(FixtureInterpolationValue::scalar))
    }
}

#[derive(Clone, Debug)]
pub(super) enum FixtureInterpolationValue {
    String { string: String },
    Integer { int: i64 },
    Float { float: f64 },
    Boolean { r#bool: bool },
}

impl<'de> Deserialize<'de> for FixtureInterpolationValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let tagged = TaggedInterpolationValue::deserialize(deserializer)?;
        match (tagged.string, tagged.int, tagged.float, tagged.r#bool) {
            (Some(string), None, None, None) => Ok(Self::String { string }),
            (None, Some(int), None, None) => Ok(Self::Integer { int }),
            (None, None, Some(float), None) if float.is_finite() => Ok(Self::Float { float }),
            (None, None, Some(_), None) => Err(serde::de::Error::custom(
                "interpolation float must be finite",
            )),
            (None, None, None, Some(r#bool)) => Ok(Self::Boolean { r#bool }),
            _ => Err(serde::de::Error::custom(
                "interpolation value must contain exactly one of string, int, float, or bool",
            )),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaggedInterpolationValue {
    #[serde(default)]
    string: Option<String>,
    #[serde(default)]
    int: Option<i64>,
    #[serde(default)]
    float: Option<f64>,
    #[serde(default)]
    r#bool: Option<bool>,
}

impl FixtureInterpolationValue {
    fn scalar(&self) -> ScalarValue {
        match self {
            Self::String { string } => ScalarValue::String(string.clone()),
            Self::Integer { int } => ScalarValue::Integer(*int),
            Self::Float { float } => ScalarValue::Float(*float),
            Self::Boolean { r#bool } => ScalarValue::Boolean(*r#bool),
        }
    }
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
