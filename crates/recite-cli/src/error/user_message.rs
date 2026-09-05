use crate::error::CliError;
use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};
use recite_config::ConfigError;
use recite_ui::{UiArg, UiArgs};

#[cfg(test)]
mod tests;

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
            Self::UserConfig { source } => user_config_message(source, messages),
            Self::ProjectDiscovery { source } => {
                messages.format(MsgId::CliErrorGeneric, [("message", source.to_string())])
            }
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
            Self::DialogueCatalogPluralFormsConflict {
                path,
                locale,
                existing,
                provided,
            } => messages.format(
                MsgId::CliErrorDialogueCatalogPluralFormsConflict,
                [
                    ("path", display_path(path)),
                    ("locale", locale.clone()),
                    ("existing", existing.clone()),
                    ("provided", provided.clone()),
                ],
            ),
            Self::DialogueCatalogMalformed { path, line, reason } => {
                let args = UiArgs::from([
                    ("path".to_owned(), UiArg::from(display_path(path))),
                    ("line".to_owned(), UiArg::from(*line)),
                    (
                        "reason".to_owned(),
                        UiArg::from(reason.user_message(messages)),
                    ),
                ]);
                messages.format_args(MsgId::CliErrorDialogueCatalogMalformed, &args)
            }
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
            Self::Bench { message } => {
                messages.format(MsgId::CliErrorBench, [("message", message.clone())])
            }
            Self::Benchmark(error) => {
                messages.format(MsgId::CliErrorBenchmark, [("message", error.to_string())])
            }
            Self::DecodeAsset { path, source } => messages.format(
                MsgId::CliErrorDecodeAsset,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::Diagnostics => messages.text(MsgId::CliErrorDiagnostics),
            Self::DiagnosticRendering { source } => messages.format(
                MsgId::CliErrorDiagnosticRendering,
                [("source", source.clone())],
            ),
            Self::FixtureChoiceIndexOutOfRange {
                index,
                choice_count,
                prompt_keys,
            } => {
                let args = UiArgs::from([
                    ("index".to_owned(), UiArg::from(*index)),
                    ("prompt_keys".to_owned(), UiArg::from(prompt_keys.join("|"))),
                    ("choice_count".to_owned(), UiArg::from(*choice_count)),
                ]);
                messages.format_args(MsgId::CliErrorFixtureChoiceIndex, &args)
            }
            Self::FixtureChoiceNotInPrompt {
                choice,
                prompt_keys,
            } => messages.format(
                MsgId::CliErrorFixtureChoiceNotInPrompt,
                [
                    ("choice", choice.clone()),
                    ("prompt_keys", prompt_keys.join("|")),
                ],
            ),
            Self::AmbiguousFixtureChoice {
                block,
                prompt_count,
            } => {
                let args = UiArgs::from([
                    ("block".to_owned(), UiArg::from(block.clone())),
                    ("prompt_count".to_owned(), UiArg::from(*prompt_count)),
                ]);
                messages.format_args(MsgId::CliErrorAmbiguousFixtureChoice, &args)
            }
            Self::FixtureToml { path, source } => messages.format(
                MsgId::CliErrorFixtureToml,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::AssetMetadata { path, source } => messages.format(
                MsgId::CliErrorAssetMetadata,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::AssetNotFile { path } => {
                messages.format(MsgId::CliErrorAssetNotFile, [("path", display_path(path))])
            }
            Self::MissingPath(path) => {
                messages.format(MsgId::CliErrorMissingPath, [("path", display_path(path))])
            }
            Self::InvalidProjectRoot(path) => messages.format(
                MsgId::CliErrorGeneric,
                [(
                    "message",
                    format!(
                        "input project root is not a directory: {}",
                        display_path(path)
                    ),
                )],
            ),
            Self::MissingFixtureChoice { prompt_keys } => messages.format(
                MsgId::CliErrorMissingFixtureChoice,
                [("prompt_keys", prompt_keys.join("|"))],
            ),
            Self::NoInputs => messages.text(MsgId::CliErrorNoInputs),
            Self::OutputOverwritesInput { output, input } => messages.format(
                MsgId::CliErrorOutputOverwritesInput,
                [
                    ("input", display_path(input)),
                    ("output", display_path(output)),
                ],
            ),
            Self::BlockingEffectNeedsAcknowledgement { effect } => {
                messages.format(MsgId::CliErrorBlockingEffect, [("effect", effect.clone())])
            }
            Self::BenchJson(error) => {
                messages.format(MsgId::CliErrorBenchJson, [("error", error.to_string())])
            }
            Self::TraceJson(error) => {
                messages.format(MsgId::CliErrorTraceJson, [("error", error.to_string())])
            }
            Self::SchemaInspection(error) => error.to_user_message(messages),
            Self::Read { path, source } => messages.format(
                MsgId::CliErrorRead,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::ReadDir { path, source } => messages.format(
                MsgId::CliErrorReadDir,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::Write { path, source } => messages.format(
                MsgId::CliErrorWrite,
                [("path", display_path(path)), ("source", source.to_string())],
            ),
            Self::Core(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::Compile(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::CompiledValue(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::Runtime(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::Preview(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::Io(error) => {
                messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())])
            }
            Self::MalformedCompiledAsset { reason } => messages.format(
                MsgId::CliErrorMalformedCompiledAsset,
                [("reason", reason.clone())],
            ),
            Self::DiagnosticCodeMalformed { code, suggestion } => {
                let args = UiArgs::from([
                    ("code".to_owned(), UiArg::from(code.clone())),
                    (
                        "suggestion".to_owned(),
                        UiArg::from(suggestion.clone().unwrap_or_default()),
                    ),
                    (
                        "has_suggestion".to_owned(),
                        UiArg::Boolean(suggestion.is_some()),
                    ),
                ]);
                messages.format_args(MsgId::CliErrorDiagnosticCodeMalformed, &args)
            }
            Self::DiagnosticCodeUnknown { code, suggestion } => {
                let args = UiArgs::from([
                    ("code".to_owned(), UiArg::from(code.clone())),
                    (
                        "suggestion".to_owned(),
                        UiArg::from(suggestion.clone().unwrap_or_default()),
                    ),
                    (
                        "has_suggestion".to_owned(),
                        UiArg::Boolean(suggestion.is_some()),
                    ),
                ]);
                messages.format_args(MsgId::CliErrorDiagnosticCodeUnknown, &args)
            }
            Self::UiCatalog { source } => {
                messages.format(MsgId::CliErrorUiCatalog, [("source", source.clone())])
            }
            Self::Watch { message } => {
                messages.format(MsgId::CliErrorWatch, [("message", message.clone())])
            }
            Self::WatchPreparation { source } => {
                messages.format(MsgId::CliErrorWatch, [("message", source.to_string())])
            }
            Self::WatchPublisher { source } => {
                messages.format(MsgId::CliErrorWatch, [("message", source.to_string())])
            }
            Self::WatchCoordinator { source, .. } => {
                messages.format(MsgId::CliErrorWatch, [("message", source.to_string())])
            }
            Self::WatchRecovery { source, .. } => source.to_user_message(messages),
        }
    }
}
fn user_config_message(error: &ConfigError, messages: &Messages) -> String {
    match error {
        ConfigError::Read { path, message } => messages.format(
            MsgId::CliErrorUiConfigRead,
            [("path", display_path(path)), ("source", message.clone())],
        ),
        ConfigError::Malformed { path, message } => messages.format(
            MsgId::CliErrorUiConfigToml,
            [("path", display_path(path)), ("source", message.clone())],
        ),
        ConfigError::InvalidLocale { path, locale } => messages.format(
            MsgId::CliErrorUiLocaleInvalid,
            [("path", display_path(path)), ("locale", locale.clone())],
        ),
        _ => messages.format(MsgId::CliErrorGeneric, [("message", error.to_string())]),
    }
}
