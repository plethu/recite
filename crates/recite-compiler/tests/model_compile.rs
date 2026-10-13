use recite_compiler::compile::{
    CompileInput, CompileOptions, compile_inputs_to_messagepack, compile_inputs_to_model,
};
use recite_core::compiled::{
    CompiledAssetHeader, CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint,
    SourceMapId, encode_compiled_dialogue_messagepack,
};

fn options(asset: &str) -> Result<CompileOptions, Box<dyn std::error::Error>> {
    Ok(CompileOptions::new(
        CompilerVersion::new("model-test")?,
        CompiledAssetId::new(asset)?,
        SourceMapId::new(format!("{asset}.map"))?,
        SchemaFingerprint::NoSchema,
    ))
}

#[test]
fn moving_tables_between_target_headers_resets_prepared_fingerprints()
-> Result<(), Box<dyn std::error::Error>> {
    let inputs = [CompileInput::new(
        "dialogue.recite",
        include_str!("../../../fixtures/recite/valid/core_language_spike.recite"),
    )];
    let report = compile_inputs_to_model(inputs.clone(), options("a.recitec")?, None)?;
    assert!(report.is_ok(), "{:?}", report.diagnostics);
    let mut payload = report.dialogue.ok_or("missing model")?.into_payload();
    let mut fingerprints = Vec::new();
    let mut outputs = Vec::new();
    for target in ["a.recitec", "b.recitec", "a.recitec"] {
        let options = options(target)?;
        payload.header = CompiledAssetHeader::messagepack_v0(
            options.compiler_version,
            options.asset_id,
            options.source_map_id,
            options.schema_fingerprint,
        );
        let dialogue = CompiledDialogue::new(payload);
        // Prepare before encoding so the test detects a stale carried cache.
        let fingerprint = dialogue
            .content_fingerprint()
            .map_err(|error| error.to_string())?
            .clone();
        let bytes = encode_compiled_dialogue_messagepack(&dialogue)?;
        assert_eq!(
            Some(&bytes),
            compile_inputs_to_messagepack(inputs.clone(), self::options(target)?)?
                .messagepack
                .as_ref()
        );
        fingerprints.push(fingerprint);
        outputs.push(bytes);
        payload = dialogue.into_payload();
    }
    assert_ne!(fingerprints[0], fingerprints[1]);
    assert_eq!(fingerprints[0], fingerprints[2]);
    assert_ne!(outputs[0], outputs[1]);
    assert_eq!(outputs[0], outputs[2]);
    Ok(())
}
