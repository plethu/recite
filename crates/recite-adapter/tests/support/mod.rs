use recite_adapter::{LoadedDialogue, StartRequest};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};
use recite_runtime::DialogueSessionOptions;

pub(super) fn asset(source: &str) -> Result<LoadedDialogue, Box<dyn std::error::Error>> {
    let report = compile_inputs(
        [CompileInput::new("test.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")?,
            CompiledAssetId::new("test.recitec")?,
            SourceMapId::new("test.recitec.map")?,
            SchemaFingerprint::NoSchema,
        ),
    )?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let compiled = report
        .asset
        .ok_or_else(|| std::io::Error::other("compiler emitted no asset"))?;
    Ok(LoadedDialogue::from_bytes(&compiled.messagepack)?)
}

pub(super) fn request(asset: &LoadedDialogue) -> StartRequest<'_> {
    StartRequest {
        asset,
        block_id: None,
        options: DialogueSessionOptions::new(),
    }
}
