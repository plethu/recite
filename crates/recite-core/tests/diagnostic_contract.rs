#![cfg(test)]

use recite_core::{
    DiagnosticArgumentType, DiagnosticArgumentValue, DiagnosticCode,
    DiagnosticPresentationContract, DiagnosticPresentationContractRegistryError,
    DiagnosticPresentationError, DiagnosticPresentationId, DiagnosticSeverity, SourcePosition,
    SourceSpan, auxiliary_contract_for, contract_for, contracts_for_code,
    migrated_diagnostic_presentation_contracts, presentation_for,
    validate_diagnostic_presentation_contracts,
    validate_migrated_diagnostic_presentation_contracts,
};

#[test]
fn migrated_parser_contracts_keep_code_and_presentation_pairs_explicit() {
    let contracts = migrated_diagnostic_presentation_contracts()
        .filter(|contract| contract.code().as_str().starts_with("RECITE_PARSE"))
        .collect::<Vec<_>>();
    let pairs = contracts
        .iter()
        .map(|contract| {
            (
                contract.code().as_str(),
                contract.presentation_id().as_str(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        pairs,
        [
            ("RECITE_PARSE001", "diagnostic-parse-001"),
            ("RECITE_PARSE002", "diagnostic-parse-002"),
            ("RECITE_PARSE003", "diagnostic-parse-003"),
            ("RECITE_PARSE005", "diagnostic-parse-005"),
            ("RECITE_PARSE007", "diagnostic-parse-007"),
            ("RECITE_PARSE008", "diagnostic-parse-008"),
            ("RECITE_PARSE010", "diagnostic-parse-010"),
            ("RECITE_PARSE011", "diagnostic-parse-011"),
            ("RECITE_PARSE012", "diagnostic-parse-012"),
            (
                "RECITE_PARSE012",
                "diagnostic-parse-012-unexpected-character"
            ),
            ("RECITE_PARSE013", "diagnostic-parse-013"),
            (
                "RECITE_PARSE013",
                "diagnostic-parse-013-unexpected-character"
            ),
            ("RECITE_PARSE014", "diagnostic-parse-014"),
            ("RECITE_PARSE015", "diagnostic-parse-015"),
            ("RECITE_PARSE016", "diagnostic-parse-016"),
            ("RECITE_PARSE017", "diagnostic-parse-017"),
            ("RECITE_PARSE018", "diagnostic-parse-018"),
            ("RECITE_PARSE034", "diagnostic-parse-034-expected-directive",),
            (
                "RECITE_PARSE034",
                "diagnostic-parse-034-expected-quoted-string",
            ),
            ("RECITE_PARSE034", "diagnostic-parse-034-missing-field"),
            ("RECITE_PARSE034", "diagnostic-parse-034-duplicate-field"),
            (
                "RECITE_PARSE034",
                "diagnostic-parse-034-quoted-without-field",
            ),
            (
                "RECITE_PARSE034",
                "diagnostic-parse-034-unexpected-trailing-text",
            ),
            (
                "RECITE_PARSE034",
                "diagnostic-parse-034-unterminated-quoted-string",
            ),
            ("RECITE_PARSE034", "diagnostic-parse-034-unsupported-escape",),
            (
                "RECITE_PARSE034",
                "diagnostic-parse-034-invalid-field-order",
            ),
        ]
    );

    let parse012 =
        contracts_for_code(&DiagnosticCode::new_static("RECITE_PARSE012")).collect::<Vec<_>>();
    assert_eq!(parse012.len(), 2);
    assert_eq!(
        parse012[0]
            .arguments()
            .iter()
            .map(|argument| (argument.name(), argument.argument_type()))
            .collect::<Vec<_>>(),
        [("reason", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        parse012[1]
            .arguments()
            .iter()
            .map(|argument| (argument.name(), argument.argument_type()))
            .collect::<Vec<_>>(),
        [("character", DiagnosticArgumentType::String)]
    );
    assert_eq!(
        contracts
            .iter()
            .map(|contract| contract.presentation_id())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        contracts.len()
    );
}

#[test]
fn producer_contract_rejects_missing_extra_and_wrong_typed_arguments() {
    let code = DiagnosticCode::new_static("RECITE_PARSE012");
    let contract = contract_for(
        &code,
        &DiagnosticPresentationId::new_static("diagnostic-parse-012"),
    )
    .expect("base dynamic parser contract is registered");

    assert!(matches!(
        contract.presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>(),),
        Err(DiagnosticPresentationError::MissingArgument(name)) if name == "reason"
    ));
    assert!(matches!(
        contract.presentation([
            ("reason", DiagnosticArgumentValue::String("invalid_mode".into())),
            ("unused", DiagnosticArgumentValue::Boolean(false)),
        ]),
        Err(DiagnosticPresentationError::ExtraArgument(name)) if name == "unused"
    ));
    assert!(matches!(
        contract.presentation([("reason", DiagnosticArgumentValue::Integer(1))]),
        Err(DiagnosticPresentationError::ArgumentTypeMismatch {
            name,
            expected: DiagnosticArgumentType::String,
            actual: DiagnosticArgumentType::Integer,
        }) if name == "reason"
    ));
}

#[test]
fn code_and_presentation_id_must_be_a_registered_pair() {
    let parse012 = DiagnosticCode::new_static("RECITE_PARSE012");
    let parse013_id = DiagnosticPresentationId::new_static("diagnostic-parse-013");
    assert!(contract_for(&parse012, &parse013_id).is_none());
    assert!(matches!(
        presentation_for(&parse012, &parse013_id, std::iter::empty::<(&str, DiagnosticArgumentValue)>()),
        Err(DiagnosticPresentationError::UnknownContract { code, presentation_id })
            if code == "RECITE_PARSE012" && presentation_id == "diagnostic-parse-013"
    ));
}

#[test]
fn first_party_registry_invariants_are_validated() {
    validate_migrated_diagnostic_presentation_contracts().expect("valid parser registry");
    assert!(
        auxiliary_contract_for(&DiagnosticPresentationId::new_static(
            "diagnostic-validate-013-related",
        ))
        .is_some()
    );
}

#[test]
fn registry_validation_rejects_duplicate_ids_and_argument_names() {
    const NO_ARGUMENTS: &[recite_core::DiagnosticArgumentSpec] = &[];
    const DUPLICATE_ARGUMENTS: &[recite_core::DiagnosticArgumentSpec] = &[
        recite_core::DiagnosticArgumentSpec::new("reason", DiagnosticArgumentType::String),
        recite_core::DiagnosticArgumentSpec::new("reason", DiagnosticArgumentType::String),
    ];
    let first =
        DiagnosticPresentationContract::new("RECITE_TEST001", "diagnostic-test", NO_ARGUMENTS);
    let duplicate_id =
        DiagnosticPresentationContract::new("RECITE_TEST002", "diagnostic-test", NO_ARGUMENTS);
    assert!(matches!(
        validate_diagnostic_presentation_contracts([&first, &duplicate_id]),
        Err(recite_core::DiagnosticPresentationContractRegistryError::DuplicatePresentationId(
            id
        )) if id == "diagnostic-test"
    ));

    let duplicate_arguments = DiagnosticPresentationContract::new(
        "RECITE_TEST003",
        "diagnostic-test-arguments",
        DUPLICATE_ARGUMENTS,
    );
    assert!(matches!(
        validate_diagnostic_presentation_contracts([&duplicate_arguments]),
        Err(DiagnosticPresentationContractRegistryError::DuplicateArgument {
            presentation_id,
            name,
        }) if presentation_id == "diagnostic-test-arguments" && name == "reason"
    ));

    const INVALID_ARGUMENT_NAME: &[recite_core::DiagnosticArgumentSpec] =
        &[recite_core::DiagnosticArgumentSpec::new(
            "Reason",
            DiagnosticArgumentType::String,
        )];
    let invalid_argument_name = DiagnosticPresentationContract::new(
        "RECITE_TEST004",
        "diagnostic-test-invalid-argument-name",
        INVALID_ARGUMENT_NAME,
    );
    assert!(matches!(
        validate_diagnostic_presentation_contracts([&invalid_argument_name]),
        Err(DiagnosticPresentationContractRegistryError::InvalidArgumentName {
            presentation_id,
            name,
        }) if presentation_id == "diagnostic-test-invalid-argument-name" && name == "Reason"
    ));
}

#[test]
fn first_party_constructor_derives_code_and_presentation_from_contract() {
    let contract = contract_for(
        &DiagnosticCode::new_static("RECITE_PARSE012"),
        &DiagnosticPresentationId::new_static("diagnostic-parse-012"),
    )
    .expect("base dynamic parser contract is registered");
    let diagnostic = recite_core::Diagnostic::error_from_contract(
        contract,
        "malformed effect statement: invalid syntax",
        SourceSpan::point(
            "dialogue/intro.recite",
            SourcePosition::new(1, 1).expect("valid source position"),
        ),
        [(
            "reason",
            DiagnosticArgumentValue::String("other".to_owned()),
        )],
    )
    .expect("contract arguments are valid");

    assert_eq!(diagnostic.code, *contract.code());
    assert_eq!(
        diagnostic.presentation.as_ref().expect("presentation").id(),
        contract.presentation_id()
    );
    assert!(matches!(
        recite_core::Diagnostic::error_from_contract(
            contract,
            "malformed effect statement",
            SourceSpan::point(
                "dialogue/intro.recite",
                SourcePosition::new(1, 1).expect("valid source position"),
            ),
            [("reason", DiagnosticArgumentValue::Integer(1))],
        ),
        Err(DiagnosticPresentationError::ArgumentTypeMismatch { name, .. }) if name == "reason"
    ));

    let warning = recite_core::Diagnostic::from_contract(
        DiagnosticSeverity::Warning,
        contract,
        "malformed effect statement: invalid syntax",
        SourceSpan::point(
            "dialogue/intro.recite",
            SourcePosition::new(1, 1).expect("valid source position"),
        ),
        [(
            "reason",
            DiagnosticArgumentValue::String("other".to_owned()),
        )],
    )
    .expect("contract arguments are valid");
    assert_eq!(warning.severity, DiagnosticSeverity::Warning);
    assert_eq!(warning.code, *contract.code());
    assert_eq!(
        warning.presentation.as_ref().expect("presentation").id(),
        contract.presentation_id()
    );
}
