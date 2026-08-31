use std::io;
use std::path::PathBuf;

use recite_core::CompiledAssetDecodeError;

use crate::dialogue_locale::DialogueCatalogMalformedReason;
use crate::fs::display_path;

mod user_message;

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
    DiagnosticRendering {
        source: String,
    },
    DialogueCatalogConflict {
        path: PathBuf,
        locale: String,
        context: String,
        source_text: String,
    },
    DialogueCatalogPluralFormsConflict {
        path: PathBuf,
        locale: String,
        existing: String,
        provided: String,
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
    DiagnosticCodeMalformed {
        code: String,
        suggestion: Option<String>,
    },
    DiagnosticCodeUnknown {
        code: String,
        suggestion: Option<String>,
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
    AmbiguousFixtureChoice {
        block: String,
        prompt_count: usize,
    },
    FixtureToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    AssetMetadata {
        path: PathBuf,
        source: io::Error,
    },
    AssetNotFile {
        path: PathBuf,
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
    Preview(recite_runtime::PreviewError),
    BlockingEffectNeedsAcknowledgement {
        effect: String,
    },
    Bench {
        message: String,
    },
    Benchmark(recite_benchmarks::BenchmarkError),
    BenchJson(serde_json::Error),
    TraceJson(serde_json::Error),
    UserConfig {
        source: recite_config::ConfigError,
    },
    ProjectDiscovery {
        source: recite_config::ProjectDiscoveryError,
    },
    UiCatalog {
        source: String,
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
            Self::DiagnosticRendering { source } => {
                write!(formatter, "failed to render diagnostic: {source}")
            }
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
            Self::DialogueCatalogPluralFormsConflict {
                path,
                locale,
                existing,
                provided,
            } => write!(
                formatter,
                "dialogue catalog {} has conflicting Plural-Forms headers for locale `{locale}` (existing `{existing}`, provided `{provided}`)",
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
            Self::DiagnosticCodeMalformed {
                code,
                suggestion,
            } => {
                write!(
                    formatter,
                    "malformed diagnostic code `{code}`: expected an uppercase namespaced code such as RECITE_PARSE001"
                )?;
                if let Some(suggestion) = suggestion {
                    write!(formatter, "; did you mean `{suggestion}`?")?;
                }
                Ok(())
            }
            Self::DiagnosticCodeUnknown { code, suggestion } => {
                write!(formatter, "unknown diagnostic code `{code}`")?;
                if let Some(suggestion) = suggestion {
                    write!(formatter, "; did you mean `{suggestion}`?")?;
                }
                Ok(())
            }
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
            Self::AmbiguousFixtureChoice {
                block,
                prompt_count,
            } => write!(
                formatter,
                "fixture block choice `{block}` is ambiguous: the block contains {prompt_count} prompts; use a line ID"
            ),
            Self::FixtureToml { path, source } => {
                write!(
                    formatter,
                    "failed to parse fixture {}: {source}",
                    display_path(path)
                )
            }
            Self::AssetMetadata { path, source } => write!(
                formatter,
                "failed to inspect compiled asset {}: {source}",
                display_path(path)
            ),
            Self::AssetNotFile { path } => write!(
                formatter,
                "compiled asset path {} is not a regular file",
                display_path(path)
            ),
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
            Self::Preview(error) => write!(formatter, "{error}"),
            Self::BlockingEffectNeedsAcknowledgement { effect } => write!(
                formatter,
                "blocking effect `{effect}` requires [effects].auto_ack_blocking = true in the fixture"
            ),
            Self::Bench { message } => formatter.write_str(message),
            Self::Benchmark(error) => write!(formatter, "{error}"),
            Self::BenchJson(error) => write!(formatter, "failed to read or write benchmark JSON: {error}"),
            Self::TraceJson(error) => write!(formatter, "failed to encode trace JSON: {error}"),
            Self::UserConfig { source } => write!(formatter, "{source}"),
            Self::ProjectDiscovery { source } => write!(formatter, "{source}"),
            Self::UiCatalog { source } => write!(formatter, "failed to load UI text catalog: {source}"),
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

impl From<recite_benchmarks::BenchmarkError> for CliError {
    fn from(error: recite_benchmarks::BenchmarkError) -> Self {
        Self::Benchmark(error)
    }
}

impl From<recite_config::ConfigError> for CliError {
    fn from(source: recite_config::ConfigError) -> Self {
        Self::UserConfig { source }
    }
}
