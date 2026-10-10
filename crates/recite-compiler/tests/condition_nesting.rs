use recite_compiler::compile::{CompileInput, CompileOptions, CompileReport, compile_inputs};
use recite_core::compiled::{
    CompiledAssetId, CompilerVersion, MAX_COMPILED_CONDITION_DEPTH, SchemaFingerprint, SourceMapId,
    decode_compiled_dialogue_messagepack,
};

#[test]
fn unary_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    assert_compiled_round_trip(&format!(
        "{}ready()",
        "not ".repeat(MAX_COMPILED_CONDITION_DEPTH)
    ))
}

#[test]
fn grouped_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    assert_compiled_round_trip(&format!("{}ready(){}", "(".repeat(128), ")".repeat(128)))
}

#[test]
fn boolean_condition_boundary_round_trips_through_the_compiled_asset()
-> Result<(), Box<dyn std::error::Error>> {
    // Each group contributes an Or edge and an And edge to the compiled tree.
    assert_compiled_round_trip(&mixed_condition(MAX_COMPILED_CONDITION_DEPTH / 2))
}

#[test]
fn accepted_source_nesting_can_exceed_the_compiled_condition_depth()
-> Result<(), Box<dyn std::error::Error>> {
    let expression = mixed_condition(128);
    let lowered = recite_parser::parse("conditions.recite", condition_source(&expression))
        .lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let report = compile_condition(&expression)?;
    assert!(report.asset.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code.as_str(), "RECITE_VALIDATE050");
    Ok(())
}

fn mixed_condition(groups: usize) -> String {
    let mut expression = "ready()".to_owned();
    for _ in 0..groups {
        expression = format!("(ready() or ready() and {expression})");
    }
    expression
}

fn condition_source(expression: &str) -> String {
    format!(
        ":: start default\n:if {expression}\n  > line@11111111111111111111\n    Hello.\n-> END\n"
    )
}

fn compile_condition(expression: &str) -> Result<CompileReport, Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("0.0.1")?,
        CompiledAssetId::new("dialogue/main.recitec")?,
        SourceMapId::new("dialogue/main.recitec.map")?,
        SchemaFingerprint::NoSchema,
    );
    Ok(compile_inputs(
        [CompileInput::new(
            "conditions.recite",
            condition_source(expression),
        )],
        options,
    )?)
}

fn assert_compiled_round_trip(expression: &str) -> Result<(), Box<dyn std::error::Error>> {
    let report = compile_condition(expression)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report
        .asset
        .ok_or("supported condition must emit an asset")?;
    let decoded = decode_compiled_dialogue_messagepack(&asset.messagepack)?;
    assert_eq!(decoded, asset.dialogue);
    assert_eq!(decoded.clone(), decoded);
    Ok(())
}
