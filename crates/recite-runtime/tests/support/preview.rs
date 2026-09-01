use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId};

pub(crate) fn asset(source: &str) -> recite_core::CompiledDialogue {
    let compiler_version = match CompilerVersion::new("0.0.1") {
        Ok(version) => version,
        Err(error) => panic!("compiler version: {error}"),
    };
    let asset_id = match CompiledAssetId::new("dialogue/preview.recitec") {
        Ok(asset_id) => asset_id,
        Err(error) => panic!("asset id: {error}"),
    };
    let source_map_id = match SourceMapId::new("dialogue/preview.map") {
        Ok(source_map_id) => source_map_id,
        Err(error) => panic!("source map id: {error}"),
    };
    let report = match compile_inputs(
        [CompileInput::new("dialogue/preview.recite", source)],
        CompileOptions::new(
            compiler_version,
            asset_id,
            source_map_id,
            SchemaFingerprint::NoSchema,
        ),
    ) {
        Ok(report) => report,
        Err(error) => panic!("compiler report: {error}"),
    };
    assert!(
        report.diagnostics.is_empty(),
        "diagnostics: {:?}",
        report.diagnostics
    );
    match report.asset {
        Some(asset) => asset.dialogue,
        None => panic!("compiled asset missing"),
    }
}
