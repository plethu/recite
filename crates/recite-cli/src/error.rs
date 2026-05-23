use std::io;
use std::path::PathBuf;

use recite_core::CompiledAssetDecodeError;

use crate::fs::display_path;

#[derive(Debug)]
pub(crate) enum CliError {
    Core(recite_core::CoreValueError),
    Compile(recite_compiler::CompileError),
    CompiledValue(recite_core::CompiledValueError),
    DecodeAsset {
        path: PathBuf,
        source: CompiledAssetDecodeError,
    },
    Diagnostics,
    FixtureChoiceIndexOutOfRange {
        index: usize,
        choice_count: usize,
        prompt_keys: Vec<String>,
    },
    FixtureChoiceNotInPrompt {
        choice: String,
        prompt_keys: Vec<String>,
    },
    FixtureToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    Io(io::Error),
    MalformedCompiledAsset {
        reason: String,
    },
    MissingPath(PathBuf),
    MissingFixtureChoice {
        prompt_keys: Vec<String>,
    },
    NoInputs,
    OutputOverwritesInput {
        output: PathBuf,
        input: PathBuf,
    },
    Read {
        path: PathBuf,
        source: io::Error,
    },
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    Runtime(recite_runtime::DialogueError),
    BlockingEffectNeedsAcknowledgement {
        effect: String,
    },
    TraceJson(serde_json::Error),
    UnknownPrompt {
        line: Option<String>,
        choices: Vec<String>,
    },
    Write {
        path: PathBuf,
        source: io::Error,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Compile(error) => write!(formatter, "{error}"),
            Self::CompiledValue(error) => write!(formatter, "{error}"),
            Self::DecodeAsset { path, source } => {
                write!(
                    formatter,
                    "failed to decode compiled asset {}: {source}",
                    display_path(path)
                )
            }
            Self::Diagnostics => formatter.write_str("diagnostics reported"),
            Self::FixtureChoiceIndexOutOfRange {
                index,
                choice_count,
                prompt_keys,
            } => write!(
                formatter,
                "fixture choice index {index} is out of range for prompt {} with {choice_count} choices; indexes are 1-based",
                prompt_keys.join("|")
            ),
            Self::FixtureChoiceNotInPrompt {
                choice,
                prompt_keys,
            } => write!(
                formatter,
                "fixture choice `{choice}` is not in prompt {}",
                prompt_keys.join("|")
            ),
            Self::FixtureToml { path, source } => {
                write!(
                    formatter,
                    "failed to parse fixture {}: {source}",
                    display_path(path)
                )
            }
            Self::Io(error) => write!(formatter, "{error}"),
            Self::MalformedCompiledAsset { reason } => {
                write!(formatter, "malformed compiled asset: {reason}")
            }
            Self::MissingPath(path) => write!(
                formatter,
                "input path does not exist: {}",
                display_path(path)
            ),
            Self::MissingFixtureChoice { prompt_keys } => write!(
                formatter,
                "fixture is missing a [choices] entry for prompt {}; supported keys for this prompt are listed in trace prompt.identity.fixture_keys",
                prompt_keys.join("|")
            ),
            Self::NoInputs => formatter.write_str("no .recite inputs found"),
            Self::OutputOverwritesInput { output, input } => write!(
                formatter,
                "refusing to overwrite input {} with output {}",
                display_path(input),
                display_path(output)
            ),
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", display_path(path))
            }
            Self::ReadDir { path, source } => {
                write!(
                    formatter,
                    "failed to read directory {}: {source}",
                    display_path(path)
                )
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::BlockingEffectNeedsAcknowledgement { effect } => write!(
                formatter,
                "blocking effect `{effect}` requires [effects].auto_ack_blocking = true in the fixture"
            ),
            Self::TraceJson(error) => write!(formatter, "failed to encode trace JSON: {error}"),
            Self::UnknownPrompt { line, choices } => write!(
                formatter,
                "runtime emitted an unknown prompt line={} choices=[{}]",
                line.as_deref().unwrap_or("<none>"),
                choices.join(", ")
            ),
            Self::Write { path, source } => {
                write!(
                    formatter,
                    "failed to write {}: {source}",
                    display_path(path)
                )
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<recite_core::CoreValueError> for CliError {
    fn from(error: recite_core::CoreValueError) -> Self {
        Self::Core(error)
    }
}

impl From<recite_core::CompiledValueError> for CliError {
    fn from(error: recite_core::CompiledValueError) -> Self {
        Self::CompiledValue(error)
    }
}

impl From<recite_compiler::CompileError> for CliError {
    fn from(error: recite_compiler::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<recite_runtime::DialogueError> for CliError {
    fn from(error: recite_runtime::DialogueError) -> Self {
        Self::Runtime(error)
    }
}
