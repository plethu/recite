#![cfg(test)]

use recite_compiler::compile::{
    CompileError, CompileInput, CompileOptions, compile_inputs, compile_inputs_to_messagepack,
    compile_inputs_to_messagepack_with_schema, compile_inputs_to_model, compile_inputs_with_schema,
};
use recite_core::{
    compiled::{
        CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
        decode_compiled_dialogue_messagepack, encode_compiled_dialogue_messagepack,
    },
    schema::{
        ProjectSchema, ProjectionQueryFunctionDefinition, SchemaTypeRef, load_schema_manifest_str,
    },
};

fn options(schema: Option<&ProjectSchema>) -> CompileOptions {
    CompileOptions::new(
        CompilerVersion::new("messagepack-test").expect("valid compiler version"),
        CompiledAssetId::new("dialogue.recitec").expect("valid asset ID"),
        SourceMapId::new("dialogue.recitec.map").expect("valid source map ID"),
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    )
}

#[test]
fn messagepack_only_matches_full_output_for_unicode_forms_and_project_order() {
    for source in [
        include_str!("../../../fixtures/recite/valid/core_language_spike.recite"),
        include_str!("../../../fixtures/recite/valid/language_pressure.recite"),
    ] {
        let inputs = [CompileInput::new("dialogue.recite", source)];
        let full = compile_inputs(inputs.clone(), options(None)).unwrap();
        let model = compile_inputs_to_model(inputs.clone(), options(None), None).unwrap();
        assert!(model.is_ok(), "{:?}", model.diagnostics);
        let payload = compile_inputs_to_messagepack(inputs, options(None)).unwrap();
        assert!(full.is_ok(), "{:?}", full.diagnostics);
        assert!(payload.is_ok(), "{:?}", payload.diagnostics);
        let asset = full.asset.unwrap();
        assert_eq!(model.dialogue.as_ref(), Some(&asset.dialogue));
        assert_eq!(
            encode_compiled_dialogue_messagepack(&model.dialogue.unwrap()).unwrap(),
            asset.messagepack
        );
        let bytes = payload.messagepack.unwrap();
        assert_eq!(bytes, asset.messagepack);
        assert_eq!(
            decode_compiled_dialogue_messagepack(&bytes).unwrap(),
            asset.dialogue
        );
        assert!(!asset.inspection_json.is_empty());
    }

    let inputs = [
        CompileInput::new(
            "z.recite",
            ":: start default\n> hello@11111111111111111111\n  Hello.\n-> a.recite::other\n",
        ),
        CompileInput::new(
            "a.recite",
            ":: other\n> end@22222222222222222222\n  Goodbye.\n-> END\n",
        ),
    ];
    let full = compile_inputs(inputs.clone(), options(None))
        .unwrap()
        .asset
        .unwrap();
    let forward = compile_inputs_to_messagepack(inputs.clone(), options(None))
        .unwrap()
        .messagepack
        .unwrap();
    let reverse = compile_inputs_to_messagepack(inputs.clone().into_iter().rev(), options(None))
        .unwrap()
        .messagepack
        .unwrap();
    assert_eq!(forward, full.messagepack);
    assert_eq!(forward, reverse);
    let model = compile_inputs_to_model(inputs.into_iter().rev(), options(None), None)
        .unwrap()
        .dialogue
        .unwrap();
    assert_eq!(
        encode_compiled_dialogue_messagepack(&model).unwrap(),
        forward
    );
}

#[test]
fn messagepack_only_with_schema_preserves_reasons_and_canonical_schema_identity() {
    let schema = load_schema_manifest_str(
        "generated.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .unwrap();
    let inputs = [CompileInput::new(
        "dialogue.recite",
        concat!(
            ":: start default\n",
            "? ask@a94b83f2d8bd65101cc3 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "  What's the news?\n  -> END\n",
        ),
    )];
    let full = compile_inputs_with_schema(inputs.clone(), options(Some(&schema)), &schema).unwrap();
    let model =
        compile_inputs_to_model(inputs.clone(), options(Some(&schema)), Some(&schema)).unwrap();
    let payload =
        compile_inputs_to_messagepack_with_schema(inputs, options(Some(&schema)), &schema).unwrap();
    assert!(full.is_ok(), "{:?}", full.diagnostics);
    assert!(payload.is_ok(), "{:?}", payload.diagnostics);
    let bytes = payload.messagepack.unwrap();
    let asset = full.asset.unwrap();
    assert_eq!(model.dialogue, Some(asset.dialogue.clone()));
    assert_eq!(bytes, asset.messagepack);
    let decoded = decode_compiled_dialogue_messagepack(&bytes).unwrap();
    assert_eq!(decoded, asset.dialogue);
    assert_eq!(
        decoded.header.schema_fingerprint,
        schema.canonical_fingerprint()
    );
    assert!(!decoded.availability_reasons.is_empty());
}

#[test]
fn messagepack_only_preserves_parse_and_validation_diagnostics_without_output() {
    for inputs in [
        vec![
            CompileInput::new("z.recite", ":: start default\n:if (\n"),
            CompileInput::new("a.recite", ":: other\n:if (\n"),
        ],
        vec![CompileInput::new(
            "reference.recite",
            ":: start default\n-> missing\n",
        )],
        vec![CompileInput::new(
            "duplicate.recite",
            concat!(
                ":: start default\n> first@11111111111111111111\n  First.\n",
                "> second@11111111111111111111\n  Second.\n-> END\n",
            ),
        )],
    ] {
        let full = compile_inputs(inputs.clone(), options(None)).unwrap();
        let model = compile_inputs_to_model(inputs.clone(), options(None), None).unwrap();
        let payload = compile_inputs_to_messagepack(inputs, options(None)).unwrap();
        assert!(!full.diagnostics.is_empty());
        assert_eq!(payload.diagnostics, full.diagnostics);
        assert_eq!(model.diagnostics, full.diagnostics);
        assert!(!model.is_ok());
        assert!(model.dialogue.is_none());
        assert!(!payload.is_ok());
        assert!(payload.messagepack.is_none());
        assert!(full.asset.is_none());
    }
}

#[test]
fn messagepack_only_checks_schema_uses_and_unused_invalid_declarations() {
    let mut invalid = ProjectSchema::empty_v1();
    invalid.projection_queries.insert(
        "unused_lookup".into(),
        ProjectionQueryFunctionDefinition {
            params: Vec::new(),
            returns: SchemaTypeRef::Enum("missing_enum".into()),
            max_calls_per_event: Some(1),
        },
    );
    for (schema, source) in [
        (
            invalid,
            ":: start default\n> hello@11111111111111111111\n  Hello.\n-> END\n",
        ),
        (
            ProjectSchema::empty_v1(),
            ":: start default\n! immediate missing_effect(snap)\n",
        ),
    ] {
        let inputs = [CompileInput::new("dialogue.recite", source)];
        let full =
            compile_inputs_with_schema(inputs.clone(), options(Some(&schema)), &schema).unwrap();
        let model =
            compile_inputs_to_model(inputs.clone(), options(Some(&schema)), Some(&schema)).unwrap();
        let payload =
            compile_inputs_to_messagepack_with_schema(inputs, options(Some(&schema)), &schema)
                .unwrap();
        assert!(!full.diagnostics.is_empty());
        assert_eq!(payload.diagnostics, full.diagnostics);
        assert_eq!(model.diagnostics, full.diagnostics);
        assert!(!model.is_ok());
        assert!(model.dialogue.is_none());
        assert!(!payload.is_ok());
        assert!(payload.messagepack.is_none());
        assert!(full.asset.is_none());
    }
}

#[test]
fn messagepack_only_preserves_non_content_failures() {
    let full = compile_inputs([], options(None)).unwrap_err();
    let payload = compile_inputs_to_messagepack([], options(None)).unwrap_err();
    let model = compile_inputs_to_model([], options(None), None).unwrap_err();
    assert!(matches!(full, CompileError::InvalidValidatedInput(_)));
    assert_eq!(
        std::mem::discriminant(&payload),
        std::mem::discriminant(&full)
    );
    assert_eq!(payload.to_string(), full.to_string());
    assert_eq!(model.to_string(), full.to_string());
}
