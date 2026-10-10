use recite_core::compiled::{
    CompiledAssetId, CompiledDialogue, CompiledInterpolationMode, CompilerVersion,
    SchemaFingerprint, SourceMapId,
};

use crate::compile::{CompileInput, CompileOptions, compile_inputs};

use super::serialize_inspection_json;

#[test]
fn inspection_preserves_legacy_text_rows_without_exposing_storage_mode() {
    let report = compile_inputs(
        [CompileInput::new(
            "dialogue/legacy.recite",
            concat!(
                ":: start default\n",
                "> line@11111111111111111111\n",
                "  Literal text.\n",
                "  ? choice@22222222222222222222\n",
                "    Literal choice.\n",
                "    -> END\n",
            ),
        )],
        CompileOptions::new(
            CompilerVersion::new("inspection-test").expect("compiler version"),
            CompiledAssetId::new("legacy.recitec").expect("asset id"),
            SourceMapId::new("legacy.map").expect("source map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("legacy-compatible text compiles");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report.asset.expect("literal text emits an asset");
    let expected = asset.inspection_json;
    let mut payload = asset.dialogue.into_payload();
    for line in &mut payload.lines {
        line.interpolation_mode = CompiledInterpolationMode::Legacy;
    }
    for choice in &mut payload.choices {
        choice.interpolation_mode = CompiledInterpolationMode::Legacy;
    }
    let legacy = CompiledDialogue::new(payload);
    assert_eq!(
        serialize_inspection_json(&legacy).expect("legacy inspection serializes"),
        expected,
    );
}
