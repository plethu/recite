use crate::dialogue_locale::DialogueCatalogMalformedReason;
use crate::error::CliError;
use crate::schema_inspection::error::SchemaInspectionError;
use recite_compiler::authoring::BuildRunError;
use recite_config::{ConfigError, ProjectDiscoveryError};
use serde_json::{Value, json};

use super::{assert_contract, messages, wire};

#[test]
fn watch_coordinator_and_recovery_keep_watch_codes_and_wrapped_human_context() {
    assert_contract(
        CliError::WatchCoordinator {
            source: BuildRunError::MissingAuthority,
            recovery: vec![],
        },
        json!({"category":"watch","code":"watch_coordinator","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
        "coordinator has no publication authority",
    );
    assert_contract(
        CliError::WatchRecovery {
            source: Box::new(CliError::Read {
                path: "schema.json".into(),
                source: std::io::Error::other("denied"),
            }),
            recovery: vec![],
        },
        json!({"category":"watch","code":"watch_recovery","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
        "failed to read schema.json: denied",
    );
}

#[test]
fn serialization_failures_retain_boundary_and_human_source_context() {
    for (code, prefix) in [
        ("import_json", "migration JSON: "),
        ("bench_json", "failed to read or write benchmark JSON: "),
        ("trace_json", "failed to encode trace JSON: "),
    ] {
        let source = serde_json::from_str::<Value>("{").unwrap_err();
        let human = format!("{prefix}{source}");
        let error = match code {
            "import_json" => CliError::ImportJson(source),
            "bench_json" => CliError::BenchJson(source),
            _ => CliError::TraceJson(source),
        };
        let category = if code == "import_json" {
            "input"
        } else {
            "serialization"
        };
        assert_contract(
            error,
            json!({"category":category,"code":code,"operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            &human,
        );
    }
    assert_contract(
        CliError::Import(recite_import::ImportError::PositionOverflow),
        json!({"category":"input","code":"import","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
        "source position exceeds the supported range",
    );
    let source = toml::from_str::<toml::Table>("[").unwrap_err();
    let human = format!("failed to parse fixture case.toml: {source}");
    assert_contract(
        CliError::FixtureToml {
            path: "case.toml".into(),
            source,
        },
        json!({"category":"fixture","code":"fixture_toml","operation":"load_fixture","path":{"encoding":"utf8","value":"case.toml"}}),
        &human,
    );
}

#[test]
fn catalog_path_errors_preserve_catalog_authority_and_locale_context() {
    let cases = [
        (
            CliError::DialogueCatalogConflict {
                path: "cy.po".into(),
                locale: "cy".into(),
                context: "line-id".into(),
                source_text: "Hello".into(),
            },
            "dialogue_catalog_conflict",
            "dialogue catalog cy.po has conflicting translations for locale `cy`, context `line-id`, source text `Hello`",
        ),
        (
            CliError::DialogueCatalogPluralFormsConflict {
                path: "cy.po".into(),
                locale: "cy".into(),
                existing: "nplurals=2".into(),
                provided: "nplurals=3".into(),
            },
            "dialogue_catalog_plural_forms_conflict",
            "dialogue catalog cy.po has conflicting Plural-Forms headers for locale `cy` (existing `nplurals=2`, provided `nplurals=3`)",
        ),
        (
            CliError::DialogueCatalogMalformed {
                path: "cy.po".into(),
                line: 7,
                reason: DialogueCatalogMalformedReason::MissingTranslation,
            },
            "dialogue_catalog_malformed",
            "failed to parse dialogue catalog cy.po at line 7: entry is missing msgstr",
        ),
    ];
    for (error, code, human) in cases {
        assert_contract(
            error,
            json!({"category":"localisation","code":code,"operation":"load_catalog","path":{"encoding":"utf8","value":"cy.po"}}),
            human,
        );
    }
}

#[test]
fn schema_inspection_errors_do_not_reclassify_from_human_messages() {
    let cases = [
        (
            SchemaInspectionError::UnsupportedFormat {
                path: "schema.yaml".into(),
                format: "yaml".into(),
            },
            "unsupported schema inspection format `yaml` for schema.yaml",
        ),
        (
            SchemaInspectionError::Malformed {
                path: "schema.json".into(),
                format: "generated_json",
            },
            "malformed generated_json schema input schema.json",
        ),
        (
            SchemaInspectionError::InvalidSummary {
                reason: "duplicate input".into(),
            },
            "invalid schema inspection summary: duplicate input",
        ),
    ];
    for (source, human) in cases {
        assert_contract(
            CliError::SchemaInspection(source),
            json!({"category":"schema","code":"schema_inspection","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
            human,
        );
    }
    let source = serde_json::from_str::<Value>("{").unwrap_err();
    let human = format!("failed to encode schema inspection JSON: {source}");
    assert_contract(
        CliError::SchemaInspection(SchemaInspectionError::Json(source)),
        json!({"category":"schema","code":"schema_inspection","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}}),
        &human,
    );
}

#[test]
fn configuration_and_discovery_errors_keep_machine_fallbacks_and_human_paths() {
    let cases = [
        (
            ConfigError::Read {
                path: "settings.toml".into(),
                message: "denied".into(),
            },
            "could not read user config settings.toml: denied",
            "failed to read UI config settings.toml: denied",
        ),
        (
            ConfigError::Malformed {
                path: "settings.toml".into(),
                message: "invalid field".into(),
            },
            "could not parse user config settings.toml: invalid field",
            "failed to parse UI config settings.toml: invalid field",
        ),
        (
            ConfigError::InvalidLocale {
                path: "settings.toml".into(),
                locale: "bad_locale".into(),
            },
            "user config settings.toml has invalid UI locale \"bad_locale\"",
            "failed to parse UI config settings.toml: invalid [ui].locale `bad_locale`; expected a BCP-47 locale such as \"en-US\" or \"system\"",
        ),
    ];
    for (source, human, localized) in cases {
        let error = CliError::UserConfig { source };
        assert_eq!(
            wire(&error),
            json!({"category":"configuration","code":"user_config","operation":"validate","path":{"encoding":"utf8","value":"fallback.recite"}})
        );
        assert_eq!(error.to_string(), human);
        assert_eq!(error.to_user_message(&messages()), localized);
    }
    for (source, path, human) in [
        (
            ProjectDiscoveryError::NotFound {
                start: "search".into(),
            },
            "fallback.recite",
            "could not find recite.project.toml from search",
        ),
        (
            ProjectDiscoveryError::NonUtf8 {
                path: "project/recite.project.toml".into(),
            },
            "project/recite.project.toml",
            "project manifest project/recite.project.toml is not valid UTF-8",
        ),
    ] {
        assert_contract(
            CliError::ProjectDiscovery { source },
            json!({"category":"project","code":"project_discovery","operation":"validate","path":{"encoding":"utf8","value":path}}),
            human,
        );
    }
}
