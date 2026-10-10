use recite_compiler::{
    authoring::{AuthoringError, AuthoringKernel},
    compile::{CompileInput, CompileOptions, compile_inputs_with_schema},
    pot::extract_pot_with_schema,
    validation::{ProjectCompleteness, ValidationInput, validate_inputs},
};
use recite_core::{
    AvailabilityReasonId, Diagnostic,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
    schema::{
        AvailabilityReasonArgBinding, AvailabilityReasonDefinition,
        ConditionAvailabilityReasonMapping, ConditionDefinition, ConditionReturnType,
        ParameterDefinition, ProjectSchema, ProjectionQueryFunctionDefinition, SchemaLiteralValue,
        SchemaTypeRef,
    },
};

const SOURCE: &str = ":: start default\n> hello@12345678901234567890\n  Hello.\n-> END\n";

fn schema_with_unused_invalid_declaration() -> ProjectSchema {
    let mut schema = ProjectSchema::empty_v1();
    schema.projection_queries.insert(
        "unused_lookup".to_owned(),
        ProjectionQueryFunctionDefinition {
            params: Vec::new(),
            returns: SchemaTypeRef::Enum("missing_enum".to_owned()),
            max_calls_per_event: Some(1),
        },
    );
    schema
}

fn assert_invalid_declaration(diagnostics: &[Diagnostic]) {
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code.as_str(), "RECITE_SCHEMA004");
    let Some(presentation) = diagnostic.presentation.as_ref() else {
        panic!("expected structured schema diagnostic");
    };
    assert_eq!(
        presentation.id().as_str(),
        "diagnostic-schema-004-unknown-enum"
    );
    assert!(diagnostic.message.contains("unused_lookup"));
    assert!(diagnostic.message.contains("missing_enum"));
    assert!(
        diagnostic.record().is_ok(),
        "schema diagnostic is recordable"
    );
}

#[test]
fn compilation_rejects_invalid_unused_schema_declarations() {
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1").expect("compiler version"),
        CompiledAssetId::new("dialogue.recitec").expect("asset ID"),
        SourceMapId::new("dialogue.recitec.map").expect("source map ID"),
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs_with_schema(
        [CompileInput::new("dialogue.recite", SOURCE)],
        options,
        &schema_with_unused_invalid_declaration(),
    )
    .expect("invalid schema is a diagnostic failure");
    assert!(
        report.asset.is_none(),
        "invalid declarations must not reach executable output"
    );
    assert_invalid_declaration(&report.diagnostics);
}

#[test]
fn pot_extraction_rejects_invalid_unused_schema_declarations() {
    let report = extract_pot_with_schema(
        [CompileInput::new("dialogue.recite", SOURCE)],
        &schema_with_unused_invalid_declaration(),
    );
    assert!(
        report.catalog.is_none(),
        "invalid declarations must not reach localisation output"
    );
    assert_invalid_declaration(&report.diagnostics);
}

#[test]
fn batch_validation_rejects_invalid_schema_even_for_incomplete_projects() {
    let parsed = recite_parser::parse("dialogue.recite", SOURCE).lower_source_file();
    assert!(parsed.diagnostics.is_empty());
    for completeness in [
        ProjectCompleteness::Complete,
        ProjectCompleteness::Incomplete,
    ] {
        let report = validate_inputs(
            [ValidationInput::all_complete(&parsed.source_file)],
            Some(&schema_with_unused_invalid_declaration()),
            completeness,
        );
        assert_invalid_declaration(&report.diagnostics);
    }
}

#[test]
fn authoring_rejects_invalid_schema_before_accepting_documents() {
    let result = AuthoringKernel::with_schema(schema_with_unused_invalid_declaration());
    let Err(AuthoringError::InvalidSchema { diagnostics }) = result else {
        panic!("invalid declarations must not enter an authoring kernel");
    };
    assert_invalid_declaration(&diagnostics);
}

#[test]
fn compile_with_programmatic_schema_rejects_non_finite_availability_reason_literals() {
    let mut schema = ProjectSchema::empty_v1();
    let reason_id = AvailabilityReasonId::new("blocked_reason").expect("valid reason id");
    schema.availability_reasons.insert(
        reason_id.clone(),
        AvailabilityReasonDefinition {
            template: "Need {threshold}".to_owned(),
            params: vec![ParameterDefinition {
                name: "threshold".to_owned(),
                type_ref: SchemaTypeRef::Float,
            }],
            origin: None,
        },
    );
    schema.conditions.insert(
        "blocked".to_owned(),
        ConditionDefinition {
            params: Vec::new(),
            returns: ConditionReturnType::Bool,
            availability_reason: Some(ConditionAvailabilityReasonMapping {
                reason: reason_id,
                args: [(
                    "threshold".to_owned(),
                    AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(
                        "NaN".to_owned(),
                    )),
                )]
                .into(),
            }),
        },
    );

    let report = compile_inputs_with_schema(
        [CompileInput::new(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "? blocked_choice@fa18ce77aacd995c1666 requires=(blocked())\n",
                "  Blocked.\n",
                "  -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .expect("invalid native schema is a diagnostic failure");

    assert!(report.asset.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code.as_str(), "RECITE_SCHEMA001");
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("native schema presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-float-not-representable",
    );
    diagnostic
        .record()
        .expect("native schema diagnostic is recordable");
}
