use super::{ErrorOperation, structured_error};
use crate::error::CliError;
use crate::i18n::{Messages, UiLocale};
use serde_json::{Value, json};
use std::path::Path;

#[path = "tests/data_errors/tests.rs"]
mod data_errors;

#[test]
fn operational_errors_preserve_specific_paths_categories_and_human_context() {
    let cases = [
        (
            CliError::Diagnostics,
            json!({"category":"internal","code":"diagnostics","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "diagnostics reported",
        ),
        (
            CliError::DiagnosticRendering {
                source: "catalog unavailable".into(),
            },
            json!({"category":"internal","code":"diagnostic_rendering","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "failed to render diagnostic: catalog unavailable",
        ),
        (
            CliError::MissingPath("missing.recite".into()),
            json!({"category":"input","code":"missing_path","operation":"resolve_path","path":{"encoding":"utf8","value":"missing.recite"}}),
            "input path does not exist: missing.recite",
        ),
        (
            CliError::InvalidProjectRoot("plain-file".into()),
            json!({"category":"input","code":"invalid_project_root","operation":"resolve_path","path":{"encoding":"utf8","value":"plain-file"}}),
            "input project root is not a directory: plain-file",
        ),
        (
            CliError::NoInputs,
            json!({"category":"input","code":"no_inputs","operation":"collect_inputs","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "no .recite inputs found",
        ),
        (
            CliError::OutputOverwritesInput {
                output: "result.recite".into(),
                input: "source.recite".into(),
            },
            json!({"category":"input","code":"output_overwrites_input","operation":"write_output","path":{"encoding":"utf8","value":"result.recite"},"related_path":{"encoding":"utf8","value":"source.recite"}}),
            "refusing to overwrite input source.recite with output result.recite",
        ),
        (
            CliError::AssetNotFile {
                path: "directory.recitec".into(),
            },
            json!({"category":"asset","code":"asset_not_file","operation":"load_asset","path":{"encoding":"utf8","value":"directory.recitec"}}),
            "compiled asset path directory.recitec is not a regular file",
        ),
        (
            CliError::AssetMetadata {
                path: "asset.recitec".into(),
                source: std::io::Error::other("metadata denied"),
            },
            json!({"category":"asset","code":"asset_metadata","operation":"inspect_asset","path":{"encoding":"utf8","value":"asset.recitec"}}),
            "failed to inspect compiled asset asset.recitec: metadata denied",
        ),
        (
            CliError::Io(std::io::Error::other("disk denied")),
            json!({"category":"io","code":"io","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "disk denied",
        ),
        (
            CliError::Read {
                path: "source.recite".into(),
                source: std::io::Error::other("read denied"),
            },
            json!({"category":"io","code":"read","operation":"read","path":{"encoding":"utf8","value":"source.recite"}}),
            "failed to read source.recite: read denied",
        ),
        (
            CliError::ReadDir {
                path: "dialogue".into(),
                source: std::io::Error::other("read denied"),
            },
            json!({"category":"io","code":"read_directory","operation":"read_directory","path":{"encoding":"utf8","value":"dialogue"}}),
            "failed to read directory dialogue: read denied",
        ),
        (
            CliError::Write {
                path: "result.recitec".into(),
                source: std::io::Error::other("write denied"),
            },
            json!({"category":"io","code":"write","operation":"write","path":{"encoding":"utf8","value":"result.recitec"}}),
            "failed to write result.recitec: write denied",
        ),
        (
            CliError::MalformedCompiledAsset {
                reason: "missing payload".into(),
            },
            json!({"category":"asset","code":"malformed_compiled_asset","operation":"load_asset","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "malformed compiled asset: missing payload",
        ),
        (
            CliError::DecodeAsset {
                path: "bad.recitec".into(),
                source: recite_core::compiled::CompiledAssetDecodeError::MalformedAsset(
                    "bad rows".into(),
                ),
            },
            json!({"category":"asset","code":"decode_asset","operation":"load_asset","path":{"encoding":"utf8","value":"bad.recitec"}}),
            "failed to decode compiled asset bad.recitec: malformed compiled asset: bad rows",
        ),
        (
            CliError::UiCatalog {
                source: "resource absent".into(),
            },
            json!({"category":"configuration","code":"ui_catalog","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "failed to load UI text catalog: resource absent",
        ),
    ];
    for (error, wire, human) in cases {
        assert_contract(error, wire, human);
    }
}

#[test]
fn fixture_failures_keep_selection_identity_and_structured_details() {
    let cases = [
        (
            CliError::FixtureChoiceIndexOutOfRange {
                index: 4,
                choice_count: 2,
                prompt_keys: vec!["line-id".into(), "block".into()],
            },
            json!({"category":"fixture","code":"fixture_choice_index_out_of_range","operation":"select_fixture_choice","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"fixture_choice_index","index":4,"choice_count":2,"prompt_keys":["line-id","block"]}}),
            "fixture choice index 4 is out of range for prompt line-id|block with 2 choices; indexes are 1-based",
        ),
        (
            CliError::FixtureChoiceNotInPrompt {
                choice: "absent".into(),
                prompt_keys: vec!["line-id".into()],
            },
            json!({"category":"fixture","code":"fixture_choice_not_in_prompt","operation":"select_fixture_choice","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"fixture_choice","choice":"absent","prompt_keys":["line-id"]}}),
            "fixture choice `absent` is not in prompt line-id",
        ),
        (
            CliError::MissingFixtureChoice {
                prompt_keys: vec!["line-id".into(), "block".into()],
            },
            json!({"category":"fixture","code":"missing_fixture_choice","operation":"select_fixture_choice","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"missing_fixture_choice","prompt_keys":["line-id","block"]}}),
            "fixture is missing a [choices] entry for prompt line-id|block; supported keys for this prompt are listed in trace prompt.identity.fixture_keys",
        ),
        (
            CliError::BlockingEffectNeedsAcknowledgement {
                effect: "wait".into(),
            },
            json!({"category":"runtime","code":"blocking_effect_needs_acknowledgement","operation":"acknowledge_effect","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"blocking_effect","effect":"wait"}}),
            "blocking effect `wait` requires [effects].auto_ack_blocking = true in the fixture",
        ),
    ];
    for (error, wire, human) in cases {
        assert_contract(error, wire, human);
    }
    let ambiguous = CliError::AmbiguousFixtureChoice {
        block: "start".into(),
        prompt_count: 3,
    };
    assert_eq!(
        wire(&ambiguous),
        json!({"category":"fixture","code":"ambiguous_fixture_choice","operation":"select_fixture_choice","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"ambiguous_fixture","block":"start","prompt_count":3}})
    );
    assert_eq!(
        ambiguous.to_string(),
        "fixture block choice `start` is ambiguous: the block contains 3 prompts; use a line ID"
    );
    assert_eq!(
        ambiguous.to_user_message(&messages()),
        "fixture block choice `start` is ambiguous across 3 prompts; use a line ID"
    );
}

#[test]
fn localisation_and_play_errors_keep_owned_details_and_fallback_operation() {
    let cases = [
        (
            CliError::DialogueCatalogMissingLocale,
            json!({"category":"localisation","code":"dialogue_catalog_missing_locale","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "dialogue catalogs require a dialogue locale; pass --dialogue-locale for play or set [dialogue].locale in the fixture",
        ),
        (
            CliError::DialogueCatalogSpecInvalid {
                spec: "catalog.po".into(),
            },
            json!({"category":"localisation","code":"dialogue_catalog_spec_invalid","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"catalog_spec","spec":"catalog.po"}}),
            "invalid dialogue catalog `catalog.po`; expected LOCALE=PATH",
        ),
        (
            CliError::DialogueLocaleInvalid {
                field: "dialogue.locale",
                locale: "bad_locale".into(),
            },
            json!({"category":"localisation","code":"dialogue_locale_invalid","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"},"details":{"type":"locale","field":"dialogue.locale","locale":"bad_locale"}}),
            "invalid dialogue locale in dialogue.locale: `bad_locale`; expected a BCP-47 locale such as \"en-US\"",
        ),
        (
            CliError::PlayEof { field: "choice" },
            json!({"category":"unsupported","code":"play_eof","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "reached EOF while reading choice",
        ),
        (
            CliError::PlayInvalidInput("choice needs a number".into()),
            json!({"category":"unsupported","code":"play_invalid_input","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "invalid play input: choice needs a number",
        ),
        (
            CliError::PlayInterrupted,
            json!({"category":"unsupported","code":"play_interrupted","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "play interrupted",
        ),
        (
            CliError::PlayTuiRequiresTerminal,
            json!({"category":"unsupported","code":"play_tui_requires_terminal","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "recite play --ui tui requires interactive stdin and stdout; use --ui plain for pipes, CI, or accessibility tools",
        ),
        (
            CliError::Bench {
                message: "case absent".into(),
            },
            json!({"category":"unsupported","code":"bench","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "case absent",
        ),
        (
            CliError::Watch {
                message: "watch unavailable".into(),
            },
            json!({"category":"watch","code":"watch","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "watch unavailable",
        ),
    ];
    for (error, expected, human) in cases {
        assert_contract(error, expected, human);
    }
}

#[test]
fn lower_level_failures_have_cli_categories_without_scraping_display_text() {
    let cases = [
        (
            CliError::Core(recite_core::CoreValueError::ZeroSourceLine),
            json!({"category":"compilation","code":"core_value","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "source line must be 1-based",
        ),
        (
            CliError::Compile(recite_compiler::compile::CompileError::Serialization(
                "cannot encode rows".into(),
            )),
            json!({"category":"compilation","code":"compile","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "cannot encode rows",
        ),
        (
            CliError::CompiledValue(recite_core::compiled::CompiledValueError::EmptyValue {
                kind: "asset id",
            }),
            json!({"category":"compilation","code":"compiled_value","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "asset id must not be empty",
        ),
        (
            CliError::Runtime(recite_runtime::DialogueError::UnknownBlock {
                block: "absent".into(),
            }),
            json!({"category":"runtime","code":"runtime","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "unknown block `absent`",
        ),
        (
            CliError::Preview(recite_runtime::preview::PreviewError::ConditionPending),
            json!({"category":"runtime","code":"preview","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "preview is waiting for a condition answer",
        ),
        (
            CliError::Benchmark(recite_benchmarks::BenchmarkError::Message(
                "unsupported workload".into(),
            )),
            json!({"category":"benchmark","code":"benchmark","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            "unsupported workload",
        ),
    ];
    for (error, expected, human) in cases {
        assert_contract(error, expected, human);
    }
    for operation in [ErrorOperation::Run, ErrorOperation::Trace] {
        let error = CliError::Core(recite_core::CoreValueError::ZeroSourceLine);
        let projected = serde_json::to_value(structured_error(&error, operation, None)).unwrap();
        let expected_operation = if matches!(operation, ErrorOperation::Run) {
            "run"
        } else {
            "trace"
        };
        assert_eq!(
            projected,
            json!({"category":"fixture","code":"core_value","operation":expected_operation})
        );
    }
}

#[test]
fn diagnostic_suggestions_are_human_help_and_not_machine_classification() {
    for suggestion in [None, Some("RECITE_PARSE001".to_owned())] {
        let malformed = CliError::DiagnosticCodeMalformed {
            code: "parse1".into(),
            suggestion: suggestion.clone(),
        };
        let unknown = CliError::DiagnosticCodeUnknown {
            code: "RECITE_PARSE999".into(),
            suggestion: suggestion.clone(),
        };
        let suffix = if suggestion.is_some() {
            "; did you mean `RECITE_PARSE001`?"
        } else {
            ""
        };
        assert_contract(
            malformed,
            json!({"category":"input","code":"diagnostic_code_malformed","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            &format!(
                "malformed diagnostic code `parse1`: expected an uppercase namespaced code such as RECITE_PARSE001{suffix}"
            ),
        );
        assert_contract(
            unknown,
            json!({"category":"input","code":"diagnostic_code_unknown","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            &format!("unknown diagnostic code `RECITE_PARSE999`{suffix}"),
        );
    }
}

fn wire(error: &CliError) -> Value {
    serde_json::to_value(structured_error(
        error,
        ErrorOperation::Validate,
        Some(Path::new("fallback.recite")),
    ))
    .unwrap()
}

fn messages() -> Messages {
    Messages::load(&UiLocale::default()).unwrap()
}

fn assert_contract(error: CliError, expected: Value, human: &str) {
    assert_eq!(wire(&error), expected);
    assert_eq!(error.to_string(), human);
    assert_eq!(error.to_user_message(&messages()), human);
}
