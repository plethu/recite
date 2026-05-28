use std::io;
use std::path::PathBuf;

use recite_core::CompiledAssetDecodeError;

use crate::dialogue_locale::DialogueCatalogMalformedReason;
use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};

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
    DialogueCatalogConflict {
        path: PathBuf,
        locale: String,
        context: String,
        source_text: String,
    },
    DialogueCatalogMalformed {
        path: PathBuf,
        line: usize,
        reason: DialogueCatalogMalformedReason,
    },
    DialogueCatalogMissingLocale,
    DialogueCatalogSpecInvalid {
        spec: String,
    },
    DialogueLocaleInvalid {
        field: &'static str,
        locale: String,
    },
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
    PlayEof {
        field: &'static str,
    },
    PlayInvalidInput(String),
    PlayInterrupted,
    PlayTuiRequiresTerminal,
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
    TuiConfigRead {
        path: PathBuf,
        source: io::Error,
    },
    TuiConfigToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    UiCatalog {
        source: String,
    },
    UiLocaleInvalid {
        path: PathBuf,
        locale: String,
    },
    UnknownPrompt {
        line: Option<String>,
        choices: Vec<String>,
    },
    Watch {
        message: String,
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
            Self::DialogueCatalogConflict {
                path,
                locale,
                context,
                source_text,
            } => write!(
                formatter,
                "dialogue catalog {} has conflicting translations for locale `{locale}`, context `{context}`, source text `{source_text}`",
                display_path(path)
            ),
            Self::DialogueCatalogMalformed {
                path,
                line,
                reason,
            } => write!(
                formatter,
                "failed to parse dialogue catalog {} at line {line}: {}",
                display_path(path),
                reason.fallback_message()
            ),
            Self::DialogueCatalogMissingLocale => formatter.write_str(
                "dialogue catalogs require a dialogue locale; pass --dialogue-locale for play or set [dialogue].locale in the fixture",
            ),
            Self::DialogueCatalogSpecInvalid { spec } => write!(
                formatter,
                "invalid dialogue catalog `{spec}`; expected LOCALE=PATH"
            ),
            Self::DialogueLocaleInvalid { field, locale } => write!(
                formatter,
                "invalid dialogue locale in {field}: `{locale}`; expected a BCP-47 locale such as \"en-US\""
            ),
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
            Self::PlayEof { field } => {
                write!(formatter, "reached EOF while reading {field}")
            }
            Self::PlayInvalidInput(message) => write!(formatter, "invalid play input: {message}"),
            Self::PlayInterrupted => formatter.write_str("play interrupted"),
            Self::PlayTuiRequiresTerminal => formatter.write_str(
                "recite play --ui tui requires interactive stdin and stdout; use --ui plain for pipes, CI, or accessibility tools",
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
            Self::TuiConfigRead { path, source } => write!(
                formatter,
                "failed to read UI config {}: {source}",
                display_path(path)
            ),
            Self::TuiConfigToml { path, source } => write!(
                formatter,
                "failed to parse UI config {}: {source}",
                display_path(path)
            ),
            Self::UiCatalog { source } => write!(formatter, "failed to load UI text catalog: {source}"),
            Self::UiLocaleInvalid { path, locale } => write!(
                formatter,
                "failed to parse UI config {}: invalid [ui].locale `{locale}`; expected a BCP-47 locale such as \"en-US\" or \"system\"",
                display_path(path)
            ),
            Self::UnknownPrompt { line, choices } => write!(
                formatter,
                "runtime emitted an unknown prompt line={} choices=[{}]",
                line.as_deref().unwrap_or("<none>"),
                choices.join(", ")
            ),
            Self::Watch { message } => formatter.write_str(message),
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

impl CliError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::PlayEof { field } => {
                messages.format(MsgId::CliErrorPlayEof, [("field", (*field).to_owned())])
            }
            Self::PlayInvalidInput(message) => messages.format(
                MsgId::CliErrorPlayInvalidInput,
                [("message", message.clone())],
            ),
            Self::PlayInterrupted => messages.text(MsgId::CliErrorPlayInterrupted),
            Self::PlayTuiRequiresTerminal => messages.text(MsgId::CliErrorPlayTuiRequiresTerminal),
            Self::TuiConfigRead { path, source } => messages.format(
                MsgId::CliErrorUiConfigRead,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::TuiConfigToml { path, source } => messages.format(
                MsgId::CliErrorUiConfigToml,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::UiLocaleInvalid { path, locale } => messages.format(
                MsgId::CliErrorUiLocaleInvalid,
                [("path", display_path(path)), ("locale", locale.clone())],
            ),
            Self::DialogueCatalogConflict {
                path,
                locale,
                context,
                source_text,
            } => messages.format(
                MsgId::CliErrorDialogueCatalogConflict,
                [
                    ("path", display_path(path)),
                    ("locale", locale.clone()),
                    ("context", context.clone()),
                    ("source_text", source_text.clone()),
                ],
            ),
            Self::DialogueCatalogMalformed { path, line, reason } => messages.format(
                MsgId::CliErrorDialogueCatalogMalformed,
                [
                    ("path", display_path(path)),
                    ("line", line.to_string()),
                    ("reason", reason.user_message(messages)),
                ],
            ),
            Self::DialogueCatalogMissingLocale => {
                messages.text(MsgId::CliErrorDialogueCatalogMissingLocale)
            }
            Self::DialogueCatalogSpecInvalid { spec } => messages.format(
                MsgId::CliErrorDialogueCatalogSpecInvalid,
                [("spec", spec.clone())],
            ),
            Self::DialogueLocaleInvalid { field, locale } => messages.format(
                MsgId::CliErrorDialogueLocaleInvalid,
                [("field", (*field).to_owned()), ("locale", locale.clone())],
            ),
            _ => self.to_string(),
        }
    }
}

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
