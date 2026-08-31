use std::path::PathBuf;

use recite_ui::{UiArg, UiArgs, UiCatalog};

use super::SchemaInspectionError;
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

fn assert_localized(
    error: SchemaInspectionError,
    id: MsgId,
    replacement: (&str, &str),
    args: UiArgs,
    expected: &str,
) {
    let default_messages = Messages::load(&UiLocale::default()).expect("default catalog loads");
    let default_text = error.to_user_message(&default_messages);
    let resource = alternate_resource(&[replacement]);
    let messages = messages_with_resource(resource.clone());
    let localized_text = error.to_user_message(&messages);
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
fn errors_use_dedicated_catalog_ids_and_arguments() {
    assert_localized(
        SchemaInspectionError::UnsupportedFormat {
            path: PathBuf::from("schema.yaml"),
            format: "yaml".to_owned(),
        },
        MsgId::CliErrorSchemaInspectionUnsupportedFormat,
        (
            "cli-error-schema-inspection-unsupported-format",
            "ALT unsupported {$path}|{$format}",
        ),
        UiArgs::from([
            ("path".to_owned(), UiArg::from("schema.yaml")),
            ("format".to_owned(), UiArg::from("yaml")),
        ]),
        "ALT unsupported schema.yaml|yaml",
    );
    assert_localized(
        SchemaInspectionError::Malformed {
            path: PathBuf::from("schema.json"),
            format: "generated_json",
        },
        MsgId::CliErrorSchemaInspectionMalformed,
        (
            "cli-error-schema-inspection-malformed",
            "ALT malformed {$path}|{$format}",
        ),
        UiArgs::from([
            ("path".to_owned(), UiArg::from("schema.json")),
            ("format".to_owned(), UiArg::from("generated_json")),
        ]),
        "ALT malformed schema.json|generated_json",
    );
    assert_localized(
        SchemaInspectionError::InvalidSummary {
            reason: "duplicate input".to_owned(),
        },
        MsgId::CliErrorSchemaInspectionInvalidSummary,
        (
            "cli-error-schema-inspection-invalid-summary",
            "ALT invalid {$reason}",
        ),
        UiArgs::from([("reason".to_owned(), UiArg::from("duplicate input"))]),
        "ALT invalid duplicate input",
    );
    assert_localized(
        SchemaInspectionError::Json(
            serde_json::from_str::<serde_json::Value>("{").expect_err("malformed JSON"),
        ),
        MsgId::CliErrorSchemaInspectionJson,
        ("cli-error-schema-inspection-json", "ALT JSON {$error}"),
        UiArgs::from([(
            "error".to_owned(),
            UiArg::from("EOF while parsing an object at line 1 column 1"),
        )]),
        "ALT JSON EOF while parsing an object at line 1 column 1",
    );
}
