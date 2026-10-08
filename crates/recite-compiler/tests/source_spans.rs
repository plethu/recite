use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
    decode_compiled_dialogue_messagepack,
};

#[test]
fn choice_requirement_source_survives_supported_line_endings_and_unicode()
-> Result<(), Box<dyn std::error::Error>> {
    for ending in ["\n", "\r\n", "\r"] {
        for final_ending in ["", ending] {
            let source = [
                ":: start default",
                "? ask@11111111111111111111 hint=\"é🦀\" requires=(ready(\"café 🦀\"))",
                "  Ask about the café.",
                "  -> END",
            ]
            .join(ending)
                + final_ending;
            assert_requirement_source(&source, "requires=(ready(\"café 🦀\"))")?;
        }
    }
    Ok(())
}

#[test]
fn choice_requirement_after_a_long_unicode_header_keeps_its_complete_span()
-> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        ":: start default\n? ask@11111111111111111111 hint=\"{}é\" requires=(ready())\n  Ask.\n  -> END",
        "x".repeat(100_000),
    );
    assert_requirement_source(&source, "requires=(ready())")
}

fn assert_requirement_source(
    source: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1")?,
        CompiledAssetId::new("dialogue/main.recitec")?,
        SourceMapId::new("dialogue/main.recitec.map")?,
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs([CompileInput::new("main.recite", source)], options)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report.asset.ok_or("source must emit an asset")?;
    let choice = asset
        .dialogue
        .choices
        .first()
        .ok_or("asset must contain a choice")?;
    assert_eq!(
        choice.availability_requirement_source_text.as_deref(),
        Some(expected),
    );
    let span = &asset
        .dialogue
        .source_maps
        .get(choice.source_map.as_u32() as usize)
        .ok_or("choice must have an in-bounds source map")?
        .span;
    assert_eq!(span.file, "main.recite");
    assert_eq!((span.start.line(), span.start.column()), (2, 1));
    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)?;
    assert_eq!(decoded.choices, asset.dialogue.choices);
    Ok(())
}
