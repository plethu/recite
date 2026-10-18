use recite_compiler::{
    authoring::{BuildGeneration, BuildInput, BuildRequest, SnapshotGeneration},
    compile::{
        CompileInput, CompileOptions, compile_build_request_to_model, compile_inputs_to_model,
        validate_build_request,
    },
};
use recite_core::{
    DocumentKey,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};

#[test]
fn immutable_captured_sources_match_raw_compilation_including_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let authored = include_str!("../../../fixtures/recite/valid/core_language_spike.recite");
    for source in [authored.to_owned(), authored.replace('\n', "\r\n"), authored.replace('\n', "\r"), ":: start default\n:if broken(\n  malformed\n".into(), ":: start default\n-> missing.recite::nope\n".into(), ":: start default\n> a@11111111111111111111\n  世界, café.\n> b@11111111111111111111\n  Duplicate.\n-> END\n".into()] {
        let request = BuildRequest::new(
            BuildGeneration::initial(), SnapshotGeneration::initial(),
            [BuildInput::saved_source(DocumentKey::new("dialogue.recite")?, &source)],
        )?;
        let options = CompileOptions::new(CompilerVersion::new("captured-test")?, CompiledAssetId::new("dialogue")?, SourceMapId::new("dialogue.map")?, SchemaFingerprint::NoSchema);
        let expected = compile_inputs_to_model([CompileInput::new("dialogue.recite", source)], options.clone(), None)?;
        assert_eq!(compile_build_request_to_model(&request, options.clone())?, expected);
        assert_eq!(validate_build_request(&request), expected.diagnostics);
        assert_eq!(validate_build_request(&request.clone()), expected.diagnostics);
    }
    Ok(())
}
