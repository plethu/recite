use std::path::PathBuf;

use recite_ui::{UiArg, UiArgs, UiCatalog};

use crate::dialogue_locale::DialogueCatalogMalformedReason;
use crate::error::CliError;
use crate::i18n::{Messages, MsgId, UiLocale};

fn locale(value: &str) -> unic_langid::LanguageIdentifier {
    value.parse().expect("test locale")
}

fn alternate_resource(overrides: &[(&str, &str)]) -> String {
    let mut seen = vec![false; overrides.len()];
    let mut resource = String::new();
    let mut skip_original_selector = false;
    for line in recite_ui::DEFAULT_RESOURCE.lines() {
        if skip_original_selector {
            if line == "}" {
                skip_original_selector = false;
            }
            continue;
        }
        let Some((id, value)) = line.split_once(" = ") else {
            resource.push_str(line);
            resource.push('\n');
            continue;
        };
        if let Some((index, (_, replacement))) = overrides
            .iter()
            .enumerate()
            .find(|(_, (override_id, _))| *override_id == id)
        {
            resource.push_str(id);
            resource.push_str(" = ");
            resource.push_str(replacement);
            resource.push('\n');
            if value.contains("->") {
                skip_original_selector = true;
            }
            seen[index] = true;
        } else {
            resource.push_str(line);
            resource.push('\n');
        }
    }
    assert!(seen.into_iter().all(|was_seen| was_seen));
    resource
}

fn messages_with_resource(resource: String) -> Messages {
    Messages::from_resources(locale("en-US"), [(locale("en-US"), resource)])
        .expect("alternate catalog loads")
}

fn assert_localized<F>(
    make_error: F,
    id: MsgId,
    replacement: (&str, &str),
    args: UiArgs,
    expected: &str,
) where
    F: Fn() -> CliError,
{
    let default_messages = Messages::load(&UiLocale::default()).expect("default catalog loads");
    let default_text = make_error().to_user_message(&default_messages);
    let resource = alternate_resource(&[replacement]);
    let messages = messages_with_resource(resource.clone());
    let localized_text = make_error().to_user_message(&messages);
    assert_ne!(localized_text, default_text);
    assert_eq!(localized_text, expected);

    let catalog = UiCatalog::from_resources(locale("en-US"), [(locale("en-US"), resource)])
        .expect("alternate catalog validates");
    let formatted = catalog
        .format_checked(id, &args)
        .expect("declared typed arguments accepted");
    assert!(
        expected.contains(&formatted),
        "catalog output {formatted:?} is not present in wrapper output {expected:?}"
    );
}

#[test]
fn first_party_error_wrappers_use_dedicated_catalog_ids_and_arguments() {
    assert_localized(
        || CliError::DialogueCatalogPluralFormsConflict {
            path: PathBuf::from("catalog.po"),
            locale: "fr".to_owned(),
            existing: "nplurals=2".to_owned(),
            provided: "nplurals=3".to_owned(),
        },
        MsgId::CliErrorDialogueCatalogPluralFormsConflict,
        (
            "cli-error-dialogue-catalog-plural-forms-conflict",
            "ALT plural {$path}|{$locale}|{$existing}|{$provided}",
        ),
        UiArgs::from([
            ("path".to_owned(), UiArg::from("catalog.po")),
            ("locale".to_owned(), UiArg::from("fr")),
            ("existing".to_owned(), UiArg::from("nplurals=2")),
            ("provided".to_owned(), UiArg::from("nplurals=3")),
        ]),
        "ALT plural catalog.po|fr|nplurals=2|nplurals=3",
    );
    assert_localized(
        || CliError::DiagnosticRendering {
            source: "catalog resolution failed".to_owned(),
        },
        MsgId::CliErrorDiagnosticRendering,
        ("cli-error-diagnostic-rendering", "ALT rendering {$source}"),
        UiArgs::from([(
            "source".to_owned(),
            UiArg::from("catalog resolution failed"),
        )]),
        "ALT rendering catalog resolution failed",
    );
    assert_localized(
        || CliError::AssetMetadata {
            path: PathBuf::from("dialogue.recitec"),
            source: std::io::Error::other("metadata failed"),
        },
        MsgId::CliErrorAssetMetadata,
        ("cli-error-asset-metadata", "ALT metadata {$path}|{$source}"),
        UiArgs::from([
            ("path".to_owned(), UiArg::from("dialogue.recitec")),
            ("source".to_owned(), UiArg::from("metadata failed")),
        ]),
        "ALT metadata dialogue.recitec|metadata failed",
    );
    assert_localized(
        || CliError::AssetNotFile {
            path: PathBuf::from("dialogue.recitec"),
        },
        MsgId::CliErrorAssetNotFile,
        ("cli-error-asset-not-file", "ALT not-file {$path}"),
        UiArgs::from([("path".to_owned(), UiArg::from("dialogue.recitec"))]),
        "ALT not-file dialogue.recitec",
    );
    assert_localized(
        || CliError::MalformedCompiledAsset {
            reason: "truncated payload".to_owned(),
        },
        MsgId::CliErrorMalformedCompiledAsset,
        (
            "cli-error-malformed-compiled-asset",
            "ALT malformed {$reason}",
        ),
        UiArgs::from([("reason".to_owned(), UiArg::from("truncated payload"))]),
        "ALT malformed truncated payload",
    );
    assert_localized(
        || CliError::DiagnosticCodeMalformed {
            code: "recite_parse001".to_owned(),
            suggestion: Some("RECITE_PARSE001".to_owned()),
        },
        MsgId::CliErrorDiagnosticCodeMalformed,
        (
            "cli-error-diagnostic-code-malformed",
            "ALT malformed-code {$code}|{$suggestion}|{$has_suggestion}",
        ),
        UiArgs::from([
            ("code".to_owned(), UiArg::from("recite_parse001")),
            ("suggestion".to_owned(), UiArg::from("RECITE_PARSE001")),
            ("has_suggestion".to_owned(), UiArg::from(true)),
        ]),
        "ALT malformed-code recite_parse001|RECITE_PARSE001|true",
    );
    assert_localized(
        || CliError::DiagnosticCodeUnknown {
            code: "RECITE_PARSE999".to_owned(),
            suggestion: None,
        },
        MsgId::CliErrorDiagnosticCodeUnknown,
        (
            "cli-error-diagnostic-code-unknown",
            "ALT unknown-code {$code}|{$suggestion}|{$has_suggestion}",
        ),
        UiArgs::from([
            ("code".to_owned(), UiArg::from("RECITE_PARSE999")),
            ("suggestion".to_owned(), UiArg::from("")),
            ("has_suggestion".to_owned(), UiArg::from(false)),
        ]),
        "ALT unknown-code RECITE_PARSE999||false",
    );
    assert_localized(
        || CliError::UiCatalog {
            source: "resource unavailable".to_owned(),
        },
        MsgId::CliErrorUiCatalog,
        ("cli-error-ui-catalog", "ALT catalog {$source}"),
        UiArgs::from([("source".to_owned(), UiArg::from("resource unavailable"))]),
        "ALT catalog resource unavailable",
    );
    assert_localized(
        || CliError::Watch {
            message: "watcher event channel closed".to_owned(),
        },
        MsgId::CliErrorWatch,
        ("cli-error-watch", "ALT watch {$message}"),
        UiArgs::from([(
            "message".to_owned(),
            UiArg::from("watcher event channel closed"),
        )]),
        "ALT watch watcher event channel closed",
    );
}

#[test]
fn malformed_catalog_reasons_use_dedicated_catalog_ids_and_arguments() {
    let cases = [
        (
            DialogueCatalogMalformedReason::InvalidPluralRule {
                detail: "plural expression divided by zero".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonInvalidPluralRule,
            (
                "cli-error-dialogue-catalog-reason-invalid-plural-rule",
                "ALT plural-rule {$detail}",
            ),
            UiArgs::from([(
                "detail".to_owned(),
                UiArg::from("plural expression divided by zero"),
            )]),
            "ALT plural-rule plural expression divided by zero",
        ),
        (
            DialogueCatalogMalformedReason::InvalidHeader {
                detail: "bad header".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonInvalidHeader,
            (
                "cli-error-dialogue-catalog-reason-invalid-header",
                "ALT header {$detail}",
            ),
            UiArgs::from([("detail".to_owned(), UiArg::from("bad header"))]),
            "ALT header bad header",
        ),
        (
            DialogueCatalogMalformedReason::InvalidStableId {
                value: "bad-context".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonInvalidStableId,
            (
                "cli-error-dialogue-catalog-reason-invalid-stable-id",
                "ALT stable-id {$value}",
            ),
            UiArgs::from([("value".to_owned(), UiArg::from("bad-context"))]),
            "ALT stable-id bad-context",
        ),
        (
            DialogueCatalogMalformedReason::DuplicateField {
                field: "msgid".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonDuplicateField,
            (
                "cli-error-dialogue-catalog-reason-duplicate-field",
                "ALT duplicate-field {$field}",
            ),
            UiArgs::from([("field".to_owned(), UiArg::from("msgid"))]),
            "ALT duplicate-field msgid",
        ),
        (
            DialogueCatalogMalformedReason::DuplicateEntry {
                key: "context|source".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonDuplicateEntry,
            (
                "cli-error-dialogue-catalog-reason-duplicate-entry",
                "ALT duplicate-entry {$key}",
            ),
            UiArgs::from([("key".to_owned(), UiArg::from("context|source"))]),
            "ALT duplicate-entry context|source",
        ),
        (
            DialogueCatalogMalformedReason::InvalidFieldOrder {
                detail: "msgid before msgctxt".to_owned(),
            },
            MsgId::CliErrorDialogueCatalogReasonInvalidFieldOrder,
            (
                "cli-error-dialogue-catalog-reason-invalid-field-order",
                "ALT field-order {$detail}",
            ),
            UiArgs::from([("detail".to_owned(), UiArg::from("msgid before msgctxt"))]),
            "ALT field-order msgid before msgctxt",
        ),
    ];

    for (reason, id, replacement, args, expected_reason) in cases {
        assert_localized(
            || CliError::DialogueCatalogMalformed {
                path: PathBuf::from("catalog.po"),
                line: 7,
                reason: reason.clone(),
            },
            id,
            replacement,
            args,
            &format!("failed to parse dialogue catalog catalog.po at line 7: {expected_reason}"),
        );
    }
}
