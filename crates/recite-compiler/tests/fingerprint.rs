use recite_compiler::{
    CompileInput, CompileOptions, CompiledAssetOutput, compile_inputs, compile_inputs_with_schema,
};
use recite_core::{
    AvailabilityReasonId, CompiledArgument, CompiledEffectMode, CompiledStatementKind, ScalarValue,
    Value,
};
use recite_core::{
    CompiledAssetId, CompilerVersion, ProjectSchema, SchemaFingerprint, SourceMapId,
    canonical_compiled_dialogue_fingerprint, encode_compiled_dialogue_messagepack,
    load_schema_manifest_str,
};
use std::path::PathBuf;

#[test]
fn full_revision_sees_schema_payload_with_reused_header_identity() -> Result<(), String> {
    let first = schema()?;
    let mut second = first.clone();
    let reason_id =
        AvailabilityReasonId::new("innkeeper_trust_hint").map_err(|error| error.to_string())?;
    second
        .availability_reasons
        .get_mut(&reason_id)
        .ok_or_else(|| "fixture reason".to_owned())?
        .template = "A different reason.".to_owned();
    let input = CompileInput::new(
        "dialogue/start.recite",
        ":: start default\n? ask@a94b83f2d8bd65101cc3 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n  Ask.\n  -> END\n",
    );
    let first = compile_with_schema(input.clone(), &first)?;
    let second = compile_with_schema(input, &second)?;
    assert_ne!(first.dialogue, second.dialogue);
    assert_ne!(
        canonical_compiled_dialogue_fingerprint(&first.dialogue)
            .map_err(|error| error.to_string())?,
        canonical_compiled_dialogue_fingerprint(&second.dialogue)
            .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn schema() -> Result<ProjectSchema, String> {
    load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .ok_or_else(|| "valid schema fixture did not load".to_owned())
}

fn compile_with_schema(
    input: CompileInput,
    schema: &ProjectSchema,
) -> Result<recite_compiler::CompiledAssetOutput, String> {
    let report = compile_inputs_with_schema(
        [input],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").map_err(|error| error.to_string())?,
            CompiledAssetId::new("dialogue/main.recitec").map_err(|error| error.to_string())?,
            SourceMapId::new("dialogue/main.recitec.map").map_err(|error| error.to_string())?,
            SchemaFingerprint::NoSchema,
        ),
        schema,
    )
    .map_err(|error| error.to_string())?;
    report
        .asset
        .ok_or_else(|| format!("compile diagnostics: {:?}", report.diagnostics))
}

#[test]
fn compiler_output_uses_the_core_encoder_byte_for_byte() -> Result<(), String> {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite")?;
    let encoded =
        encode_compiled_dialogue_messagepack(&asset.dialogue).map_err(|error| error.to_string())?;
    assert_eq!(encoded, asset.messagepack);
    Ok(())
}

#[test]
fn full_payload_fingerprint_changes_for_representative_compiled_tables() -> Result<(), String> {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite")?;
    let original = canonical_compiled_dialogue_fingerprint(&asset.dialogue)
        .map_err(|error| error.to_string())?;

    let mut line = asset.dialogue.clone();
    line.lines[0].source_text.push('!');
    line.lines[0].authored_source_text.push('!');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&line).map_err(|error| error.to_string())?
    );

    let mut choice = asset.dialogue.clone();
    choice.choices[0].source_text.push('!');
    choice.choices[0].authored_source_text.push('!');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&choice).map_err(|error| error.to_string())?
    );

    let mut statement = asset.dialogue.clone();
    statement.statements[0].kind = CompiledStatementKind::Line(recite_core::LineIndex::new(0));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&statement).map_err(|error| error.to_string())?
    );

    let mut effect = asset.dialogue.clone();
    effect.effects[0].function.push('D');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&effect).map_err(|error| error.to_string())?
    );

    let mut metadata = asset.dialogue.clone();
    metadata.metadata[0].value = Value::Scalar(ScalarValue::Boolean(true));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&metadata).map_err(|error| error.to_string())?
    );

    let mut argument = asset.dialogue.clone();
    argument.effects[0].args[0] = CompiledArgument::Value(ScalarValue::Integer(7));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&argument).map_err(|error| error.to_string())?
    );

    let mut mode = asset.dialogue.clone();
    mode.effects[0].mode = CompiledEffectMode::Immediate;
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&mode).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn compile_fixture(path: &str) -> Result<CompiledAssetOutput, String> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    let source = std::fs::read_to_string(source).map_err(|error| error.to_string())?;
    compile_inputs(
        [CompileInput::new(path, source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").map_err(|error| error.to_string())?,
            CompiledAssetId::new("dialogue/main.recitec").map_err(|error| error.to_string())?,
            SourceMapId::new("dialogue/main.recitec.map").map_err(|error| error.to_string())?,
            SchemaFingerprint::NoSchema,
        ),
    )
    .map_err(|error| error.to_string())?
    .asset
    .ok_or_else(|| "fixture compilation produced no asset".to_owned())
}
