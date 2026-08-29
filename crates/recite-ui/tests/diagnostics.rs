use std::collections::BTreeMap;

use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, contract_for,
    default_presentation_id_for_code, known_diagnostic_explanations,
};
use recite_ui::{
    CatalogError, LEGACY_DIAGNOSTIC_RESOURCE, ResourceId, UiArg, UiArgType, UiCatalog, UiContract,
    UiLocale,
};

#[test]
fn every_explanation_slot_resolves_to_human_authored_english() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    for explanation in known_diagnostic_explanations() {
        let presentation = explanation.presentation();
        assert_eq!(
            catalog
                .format_presentation(&presentation.meaning)
                .expect("meaning presentation"),
            explanation.meaning
        );
        assert_eq!(
            presentation.common_causes.len(),
            explanation.common_causes.len()
        );
        for (reference, expected) in presentation
            .common_causes
            .iter()
            .zip(explanation.common_causes)
        {
            assert_eq!(
                catalog.format_presentation(reference).expect("cause"),
                *expected
            );
        }
        assert_eq!(
            presentation.remediation.len(),
            explanation.remediation.len()
        );
        for (reference, expected) in presentation.remediation.iter().zip(explanation.remediation) {
            assert_eq!(
                catalog.format_presentation(reference).expect("remediation"),
                *expected
            );
        }
    }
}

#[test]
fn legacy_diagnostic_adapter_is_separate_from_localised_inventory() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    assert_eq!(
        include_str!("../resources/diagnostics.ftl")
            .matches("{$message}")
            .count(),
        1,
        "only the named legacy adapter may passthrough message prose"
    );
    assert_eq!(
        catalog
            .format_legacy_diagnostic_message("legacy producer message")
            .expect("legacy adapter"),
        "legacy producer message"
    );
    let primary = ResourceId::new("diagnostic-parse-001").expect("primary ID");
    assert_eq!(
        catalog
            .format_resource_checked(&primary, &BTreeMap::new())
            .expect("migrated primary"),
        "expected a Recite statement header or indented prose"
    );
    assert_ne!(primary.as_str(), LEGACY_DIAGNOSTIC_RESOURCE);
}

#[test]
fn migrated_parser_contracts_resolve_their_registered_resources() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    let static_contract = contract_for(
        &DiagnosticCode::new_static("RECITE_PARSE001"),
        &DiagnosticPresentationId::new_static("diagnostic-parse-001"),
    )
    .expect("parser contract");
    let presentation = static_contract
        .presentation(std::iter::empty::<(String, DiagnosticArgumentValue)>())
        .expect("static parser presentation");
    assert_eq!(
        catalog
            .format_presentation(&presentation)
            .expect("static parser resource"),
        "expected a Recite statement header or indented prose"
    );

    let dynamic_contract = contract_for(
        &DiagnosticCode::new_static("RECITE_PARSE013"),
        &DiagnosticPresentationId::new_static("diagnostic-parse-013"),
    )
    .expect("dynamic parser contract");
    let presentation = dynamic_contract
        .presentation([(
            "reason",
            DiagnosticArgumentValue::String("invalid_integer".to_owned()),
        )])
        .expect("dynamic parser presentation");
    assert_eq!(
        catalog
            .format_presentation(&presentation)
            .expect("dynamic parser resource"),
        "malformed condition expression: invalid integer literal"
    );

    let unexpected_character = contract_for(
        &DiagnosticCode::new_static("RECITE_PARSE013"),
        &DiagnosticPresentationId::new_static("diagnostic-parse-013-unexpected-character"),
    )
    .expect("unexpected-character parser contract");
    for (character, rendered) in [("@", "@"), ("\\'", "\\'"), ("\\\\", "\\\\"), ("\\n", "\\n")] {
        let presentation = unexpected_character
            .presentation([(
                "character",
                DiagnosticArgumentValue::String(character.to_owned()),
            )])
            .expect("unexpected-character presentation");
        assert_eq!(
            catalog
                .format_presentation(&presentation)
                .expect("unexpected-character resource"),
            format!("malformed condition expression: unexpected character '{rendered}'")
        );
    }
}

#[test]
fn schema_version_resource_preserves_arbitrary_numeric_lexemes() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    let contract = contract_for(
        &DiagnosticCode::new_static("RECITE_SCHEMA002"),
        &DiagnosticPresentationId::new_static("diagnostic-schema-002-unsupported-version"),
    )
    .expect("unsupported-version schema contract");

    for version in [
        "1.5",
        "1e-100000",
        "999999999999999999999999999999999999999999",
    ] {
        let presentation = contract
            .presentation([(
                "version",
                DiagnosticArgumentValue::String(version.to_owned()),
            )])
            .expect("arbitrary version lexeme matches contract");
        assert_eq!(
            catalog
                .format_presentation(&presentation)
                .expect("unsupported-version resource"),
            format!("unsupported schema manifest version {version}")
        );
    }
}

#[test]
fn schema_finite_variant_resources_preserve_compatibility_messages() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    for (id, arguments, expected) in [
        (
            "diagnostic-schema-001-availability-template-unterminated",
            vec![(
                "reason",
                DiagnosticArgumentValue::String("locked".to_owned()),
            )],
            "availability reason 'locked' template has invalid placeholder syntax: unterminated placeholder",
        ),
        (
            "diagnostic-schema-001-availability-template-invalid-name",
            vec![
                (
                    "reason",
                    DiagnosticArgumentValue::String("locked".to_owned()),
                ),
                (
                    "name",
                    DiagnosticArgumentValue::String("Bad-Name".to_owned()),
                ),
            ],
            "availability reason 'locked' template has invalid placeholder syntax: invalid placeholder name 'Bad-Name'",
        ),
        (
            "diagnostic-schema-001-availability-template-unescaped-closing-brace",
            vec![(
                "reason",
                DiagnosticArgumentValue::String("locked".to_owned()),
            )],
            "availability reason 'locked' template has invalid placeholder syntax: unescaped closing brace",
        ),
        (
            "diagnostic-schema-001-label-placeholder-unterminated",
            vec![
                (
                    "projector",
                    DiagnosticArgumentValue::String("hud".to_owned()),
                ),
                (
                    "output",
                    DiagnosticArgumentValue::String("label".to_owned()),
                ),
                (
                    "template_id",
                    DiagnosticArgumentValue::String("title".to_owned()),
                ),
            ],
            "projector 'hud' output 'label' presentation label 'title' has invalid placeholder syntax: unterminated placeholder",
        ),
        (
            "diagnostic-schema-001-label-placeholder-invalid-name",
            vec![
                (
                    "projector",
                    DiagnosticArgumentValue::String("hud".to_owned()),
                ),
                (
                    "output",
                    DiagnosticArgumentValue::String("label".to_owned()),
                ),
                (
                    "template_id",
                    DiagnosticArgumentValue::String("title".to_owned()),
                ),
                (
                    "name",
                    DiagnosticArgumentValue::String("Bad-Name".to_owned()),
                ),
            ],
            "projector 'hud' output 'label' presentation label 'title' has invalid placeholder syntax: invalid placeholder name 'Bad-Name'",
        ),
        (
            "diagnostic-schema-001-label-placeholder-unescaped-closing-brace",
            vec![
                (
                    "projector",
                    DiagnosticArgumentValue::String("hud".to_owned()),
                ),
                (
                    "output",
                    DiagnosticArgumentValue::String("label".to_owned()),
                ),
                (
                    "template_id",
                    DiagnosticArgumentValue::String("title".to_owned()),
                ),
            ],
            "projector 'hud' output 'label' presentation label 'title' has invalid placeholder syntax: unescaped closing brace",
        ),
        (
            "diagnostic-schema-001-producer-content-fingerprint-empty-algorithm",
            vec![],
            "manifest content_fingerprint is invalid: FingerprintAlgorithm must not be empty",
        ),
        (
            "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape",
            vec![],
            "manifest content_fingerprint is invalid: blake3 producer fingerprint must be even-length hex",
        ),
        (
            "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data",
            vec![],
            "manifest content_fingerprint is invalid: blake3 producer fingerprint must be hex",
        ),
        (
            "diagnostic-schema-001-producer-content-fingerprint-empty-digest",
            vec![],
            "manifest content_fingerprint is invalid: FingerprintDigest must not be empty",
        ),
        (
            "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length",
            vec![("actual", DiagnosticArgumentValue::Integer(3))],
            "manifest content_fingerprint is invalid: blake3 fingerprint digest must be 32 bytes, got 3",
        ),
    ] {
        let contract = contract_for(
            &DiagnosticCode::new_static("RECITE_SCHEMA001"),
            &DiagnosticPresentationId::new_static(id),
        )
        .expect("schema variant contract");
        let presentation = contract
            .presentation(arguments)
            .expect("schema variant arguments");
        assert_eq!(
            catalog
                .format_presentation(&presentation)
                .expect("resource"),
            expected
        );
    }
}

#[test]
fn diagnostic_adapter_retains_typed_argument_errors() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    let id = ResourceId::new(LEGACY_DIAGNOSTIC_RESOURCE).expect("adapter ID");
    let args = BTreeMap::from([("message".to_owned(), UiArg::Integer(3))]);
    let error = catalog
        .format_resource_checked(&id, &args)
        .expect_err("wrong diagnostic argument type");
    assert!(matches!(
        error,
        CatalogError::ArgumentTypeMismatch {
            expected: UiArgType::String,
            actual: UiArgType::Integer,
            ..
        }
    ));
}

#[test]
fn diagnostic_resource_file_rejects_missing_and_unused_slots() {
    let contract = UiContract::default();
    let diagnostic_contract = UiContract::new(
        contract
            .resources
            .iter()
            .filter(|resource| resource.id.as_str().starts_with("diagnostic-"))
            .cloned()
            .collect(),
        contract.clients.clone(),
    );
    let source = include_str!("../resources/diagnostics.ftl");
    let missing = source.replacen(
        "diagnostic-parse-001-meaning = ",
        "diagnostic-parse-001-removed = ",
        1,
    );
    let missing_error = diagnostic_contract
        .validate(&missing)
        .expect_err("missing explanation slot");
    assert!(
        missing_error
            .to_string()
            .contains("missing resource ID `diagnostic-parse-001-meaning`")
    );

    let unused = source.replacen(
        "diagnostic-parse-001-meaning = The parser",
        "diagnostic-parse-001-meaning = { $unused } The parser",
        1,
    );
    let unused_error = diagnostic_contract
        .validate(&unused)
        .expect_err("unused explanation argument");
    assert!(
        unused_error
            .to_string()
            .contains("undeclared argument `unused`")
    );
}

#[test]
fn explanation_lookup_uses_the_stable_code_registry() {
    let code = DiagnosticCode::new_static("RECITE_PARSE001");
    let explanation = recite_core::explain_diagnostic_code(&code).expect("known explanation");
    assert_eq!(
        explanation.default_code_presentation_id().as_str(),
        "diagnostic-parse-001"
    );

    let future_code = DiagnosticCode::new("RECITE_CUSTOM_LABEL").expect("valid future code");
    assert_eq!(
        default_presentation_id_for_code(&future_code).as_str(),
        "diagnostic-code-5245434954455f435553544f4d5f4c4142454c"
    );
}
