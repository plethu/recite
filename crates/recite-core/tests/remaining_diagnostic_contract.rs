use recite_core::{
    DiagnosticArgumentType, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId,
    auxiliary_contract_for, contract_for, contracts_for_code,
};

fn signature(code: &str, id: &str) -> Vec<(&'static str, DiagnosticArgumentType)> {
    let code = DiagnosticCode::new(code)
        .unwrap_or_else(|error| panic!("test diagnostic code must be valid: {error}"));
    let id = DiagnosticPresentationId::new(id)
        .unwrap_or_else(|error| panic!("test presentation ID must be valid: {error}"));
    contract_for(&code, &id)
        .unwrap_or_else(|| panic!("missing contract {code}/{id}"))
        .arguments()
        .iter()
        .map(|argument| (argument.name(), argument.argument_type()))
        .collect()
}

#[test]
fn remaining_po_contracts_cover_each_structural_cause() {
    let parse = contracts_for_code(&DiagnosticCode::new_static("RECITE_PARSE034"))
        .map(|contract| contract.presentation_id().as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        parse,
        [
            "diagnostic-parse-034-expected-directive",
            "diagnostic-parse-034-expected-quoted-string",
            "diagnostic-parse-034-missing-field",
            "diagnostic-parse-034-duplicate-field",
            "diagnostic-parse-034-quoted-without-field",
            "diagnostic-parse-034-unexpected-trailing-text",
            "diagnostic-parse-034-unterminated-quoted-string",
            "diagnostic-parse-034-unsupported-escape",
            "diagnostic-parse-034-invalid-field-order",
        ]
    );

    for id in [
        "diagnostic-parse-034-expected-directive",
        "diagnostic-parse-034-expected-quoted-string",
        "diagnostic-parse-034-quoted-without-field",
        "diagnostic-parse-034-unexpected-trailing-text",
        "diagnostic-parse-034-unterminated-quoted-string",
    ] {
        assert!(signature("RECITE_PARSE034", id).is_empty(), "{id}");
    }
    assert_eq!(
        signature("RECITE_PARSE034", "diagnostic-parse-034-missing-field"),
        [("field", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_PARSE034", "diagnostic-parse-034-duplicate-field"),
        [("field", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_PARSE034", "diagnostic-parse-034-unsupported-escape"),
        [("escape", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature(
            "RECITE_PARSE034",
            "diagnostic-parse-034-invalid-field-order"
        ),
        [("value", DiagnosticArgumentType::String)]
    );

    assert_eq!(
        signature("RECITE_ID034", "diagnostic-id-034"),
        [("context", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_ID035", "diagnostic-id-035"),
        [
            ("context", DiagnosticArgumentType::String),
            ("source_text", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_VALIDATE042", "diagnostic-validate-042"),
        [("detail", DiagnosticArgumentType::String)]
    );

    for id in [
        "diagnostic-validate-043-contiguous-arms",
        "diagnostic-validate-043-requires-plural-source",
        "diagnostic-validate-044-multiple-headers",
        "diagnostic-validate-044-invalid-plural-forms",
        "diagnostic-validate-044-plural-header-required",
    ] {
        let code = if id.contains("043") {
            "RECITE_VALIDATE043"
        } else {
            "RECITE_VALIDATE044"
        };
        assert!(signature(code, id).is_empty(), "{id}");
    }
    assert_eq!(
        signature("RECITE_VALIDATE043", "diagnostic-validate-043-expected-arm"),
        [("expected", DiagnosticArgumentType::Integer)]
    );
    assert_eq!(
        signature("RECITE_VALIDATE043", "diagnostic-validate-043-count"),
        [
            ("expected", DiagnosticArgumentType::Integer),
            ("actual", DiagnosticArgumentType::Integer),
        ]
    );
    assert_eq!(
        signature("RECITE_VALIDATE043", "diagnostic-validate-043-invalid-arm"),
        [("keyword", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-missing-colon"
        ),
        [("line", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-duplicate-or-empty"
        ),
        [("key", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-invalid-plural-rule"
        ),
        [("detail", DiagnosticArgumentType::String)]
    );
}

#[test]
fn project_and_freshness_contracts_match_producer_shapes() {
    assert_eq!(
        signature("RECITE_PROJECT001", "diagnostic-project-001"),
        [("detail", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_PROJECT002", "diagnostic-project-002"),
        [("scene_id", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_PROJECT003", "diagnostic-project-003"),
        [
            ("scene_id", DiagnosticArgumentType::String),
            ("asset", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT004", "diagnostic-project-004"),
        [
            ("scene_id", DiagnosticArgumentType::String),
            ("block", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT005", "diagnostic-project-005"),
        [("scene_id", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_PROJECT006", "diagnostic-project-006"),
        [
            ("asset", DiagnosticArgumentType::String),
            ("source", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT007", "diagnostic-project-007"),
        [
            ("asset", DiagnosticArgumentType::String),
            ("version", DiagnosticArgumentType::Integer),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT007", "diagnostic-project-007-malformed"),
        [
            ("scene_id", DiagnosticArgumentType::String),
            ("asset", DiagnosticArgumentType::String),
            ("detail", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT008", "diagnostic-project-008"),
        [
            ("scene_id", DiagnosticArgumentType::String),
            ("participant", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_PROJECT008", "diagnostic-project-008-compiled-asset"),
        [
            ("scene_id", DiagnosticArgumentType::String),
            ("participant", DiagnosticArgumentType::String),
            ("asset", DiagnosticArgumentType::String),
        ]
    );
    assert!(
        auxiliary_contract_for(&DiagnosticPresentationId::new_static(
            "diagnostic-project-002-related"
        ))
        .is_some()
    );

    assert_eq!(
        signature("RECITE_FRESH001", "diagnostic-fresh-001"),
        [
            ("asset", DiagnosticArgumentType::String),
            ("source", DiagnosticArgumentType::String),
        ]
    );
    assert_eq!(
        signature("RECITE_FRESH002", "diagnostic-fresh-002"),
        [("asset", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        signature("RECITE_FRESH003", "diagnostic-fresh-003"),
        [
            ("asset", DiagnosticArgumentType::String),
            ("version", DiagnosticArgumentType::Integer),
            ("expected", DiagnosticArgumentType::Integer),
        ]
    );
}

fn value(argument_type: DiagnosticArgumentType) -> DiagnosticArgumentValue {
    match argument_type {
        DiagnosticArgumentType::String => DiagnosticArgumentValue::String("value".to_owned()),
        DiagnosticArgumentType::Integer => DiagnosticArgumentValue::Integer(42),
        DiagnosticArgumentType::Float => DiagnosticArgumentValue::try_float(1.5)
            .unwrap_or_else(|error| panic!("test float must be finite: {error}")),
        DiagnosticArgumentType::Boolean => DiagnosticArgumentValue::Boolean(true),
    }
}

fn wrong_value(argument_type: DiagnosticArgumentType) -> DiagnosticArgumentValue {
    match argument_type {
        DiagnosticArgumentType::String => DiagnosticArgumentValue::Integer(42),
        _ => DiagnosticArgumentValue::String("wrong type".to_owned()),
    }
}

#[test]
fn every_remaining_signature_accepts_valid_and_rejects_invalid_arguments() {
    let contracts = recite_core::migrated_diagnostic_presentation_contracts()
        .filter(|contract| {
            matches!(
                contract.code().as_str(),
                "RECITE_PARSE034"
                    | "RECITE_ID034"
                    | "RECITE_ID035"
                    | "RECITE_VALIDATE042"
                    | "RECITE_VALIDATE043"
                    | "RECITE_VALIDATE044"
                    | "RECITE_PROJECT001"
                    | "RECITE_PROJECT002"
                    | "RECITE_PROJECT003"
                    | "RECITE_PROJECT004"
                    | "RECITE_PROJECT005"
                    | "RECITE_PROJECT006"
                    | "RECITE_PROJECT007"
                    | "RECITE_PROJECT008"
                    | "RECITE_FRESH001"
                    | "RECITE_FRESH002"
                    | "RECITE_FRESH003"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(contracts.len(), 36);

    for contract in contracts {
        let valid = contract
            .arguments()
            .iter()
            .map(|argument| (argument.name().to_owned(), value(argument.argument_type())))
            .collect::<Vec<_>>();
        assert!(
            contract.presentation(valid.clone()).is_ok(),
            "valid arguments rejected by {}",
            contract.presentation_id()
        );

        let mut extra = valid.clone();
        extra.push((
            "extra".to_owned(),
            DiagnosticArgumentValue::String("x".to_owned()),
        ));
        assert!(
            matches!(
                contract.presentation(extra),
                Err(recite_core::DiagnosticPresentationError::ExtraArgument(name)) if name == "extra"
            ),
            "extra argument accepted by {}",
            contract.presentation_id()
        );

        if let Some(first) = contract.arguments().first() {
            let mut missing = valid.clone();
            missing.retain(|(name, _)| name != first.name());
            assert!(
                matches!(
                    contract.presentation(missing),
                    Err(recite_core::DiagnosticPresentationError::MissingArgument(name)) if name == first.name()
                ),
                "missing argument accepted by {}",
                contract.presentation_id()
            );

            let mut wrong = valid;
            let (_, argument) = wrong
                .iter_mut()
                .find(|(name, _)| name == first.name())
                .unwrap_or_else(|| panic!("test argument must exist: {}", first.name()));
            *argument = wrong_value(first.argument_type());
            assert!(
                matches!(
                    contract.presentation(wrong),
                    Err(recite_core::DiagnosticPresentationError::ArgumentTypeMismatch { name, .. }) if name == first.name()
                ),
                "wrong argument type accepted by {}",
                contract.presentation_id()
            );
        }
    }

    let auxiliary = auxiliary_contract_for(&DiagnosticPresentationId::new_static(
        "diagnostic-project-002-related",
    ))
    .unwrap_or_else(|| panic!("missing project related contract"));
    assert!(
        auxiliary
            .presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>())
            .is_ok()
    );
    assert!(matches!(
        auxiliary.presentation([("extra", DiagnosticArgumentValue::String("x".to_owned()))]),
        Err(recite_core::DiagnosticPresentationError::ExtraArgument(name)) if name == "extra"
    ));
}
