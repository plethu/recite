use super::compile_fixture;
use recite_compiler::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    AvailabilityReasonId, CompiledArgument, CompiledAssetId, CompiledEffectMode,
    CompiledStatementKind, CompilerVersion, ProjectSchema, ScalarValue, SchemaFingerprint,
    SourceMapId, Value, canonical_compiled_dialogue_fingerprint,
    encode_compiled_dialogue_messagepack, load_schema_manifest_str,
};

#[test]
fn full_revision_sees_schema_payload_with_reused_header_identity() {
    let first = schema();
    let mut second = first.clone();
    second
        .availability_reasons
        .get_mut(&AvailabilityReasonId::new("innkeeper_trust_hint").expect("reason id"))
        .expect("fixture reason")
        .template = "A different reason.".to_owned();
    let input = CompileInput::new(
        "dialogue/start.recite",
        ":: start default\n? ask@a94b83f2d8bd65101cc3 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n  Ask.\n  -> END\n",
    );
    let first = compile_with_schema(input.clone(), &first);
    let second = compile_with_schema(input, &second);
    assert_ne!(first.dialogue, second.dialogue);
    assert_ne!(
        canonical_compiled_dialogue_fingerprint(&first.dialogue).expect("first fingerprint"),
        canonical_compiled_dialogue_fingerprint(&second.dialogue).expect("second fingerprint")
    );
}

fn schema() -> ProjectSchema {
    load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema")
}

fn compile_with_schema(
    input: CompileInput,
    schema: &ProjectSchema,
) -> recite_compiler::CompiledAssetOutput {
    compile_inputs_with_schema(
        [input],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("asset"),
            SourceMapId::new("dialogue/main.recitec.map").expect("map"),
            SchemaFingerprint::NoSchema,
        ),
        schema,
    )
    .expect("compile")
    .asset
    .expect("asset")
}

#[test]
fn compiler_output_uses_the_core_encoder_byte_for_byte() {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite");
    let encoded = encode_compiled_dialogue_messagepack(&asset.dialogue).expect("encode");
    assert_eq!(encoded, asset.messagepack);
}

#[test]
fn full_payload_fingerprint_changes_for_representative_compiled_tables() {
    let asset = compile_fixture("fixtures/recite/valid/core_language_spike.recite");
    let original = canonical_compiled_dialogue_fingerprint(&asset.dialogue).expect("fingerprint");

    let mut line = asset.dialogue.clone();
    line.lines[0].source_text.push('!');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&line).expect("line")
    );

    let mut choice = asset.dialogue.clone();
    choice.choices[0].source_text.push('!');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&choice).expect("choice")
    );

    let mut statement = asset.dialogue.clone();
    statement.statements[0].kind = CompiledStatementKind::Line(recite_core::LineIndex::new(0));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&statement).expect("statement")
    );

    let mut effect = asset.dialogue.clone();
    effect.effects[0].function.push('!');
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&effect).expect("effect")
    );

    let mut metadata = asset.dialogue.clone();
    metadata.metadata[0].value = Value::Scalar(ScalarValue::Boolean(true));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&metadata).expect("metadata")
    );

    let mut argument = asset.dialogue.clone();
    argument.effects[0].args[0] = CompiledArgument::Value(ScalarValue::Integer(7));
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&argument).expect("argument")
    );

    let mut mode = asset.dialogue.clone();
    mode.effects[0].mode = CompiledEffectMode::Immediate;
    assert_ne!(
        original,
        canonical_compiled_dialogue_fingerprint(&mode).expect("mode")
    );
}
