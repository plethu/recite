use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
    decode_compiled_dialogue_messagepack,
};

#[test]
fn unary_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    assert_compiled_round_trip(&format!("{}ready()", "not ".repeat(128)))
}

#[test]
fn grouped_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    assert_compiled_round_trip(&format!("{}ready(){}", "(".repeat(128), ")".repeat(128)))
}

#[test]
fn boolean_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    let mut expression = "ready()".to_owned();
    for _ in 0..128 {
        expression = format!("(ready() or ready() and {expression})");
    }
    assert_compiled_round_trip(&expression)
}

fn assert_compiled_round_trip(expression: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        ":: start default\n:if {expression}\n  > line@11111111111111111111\n    Hello.\n-> END\n",
    );
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1")?,
        CompiledAssetId::new("dialogue/main.recitec")?,
        SourceMapId::new("dialogue/main.recitec.map")?,
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs([CompileInput::new("conditions.recite", source)], options)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report
        .asset
        .ok_or("supported condition must emit an asset")?;
    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)?;
    assert_eq!(decoded, asset.dialogue);
    assert_eq!(decoded.clone(), decoded);
    Ok(())
}
