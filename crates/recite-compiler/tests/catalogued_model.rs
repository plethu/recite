use recite_compiler::{
    compile::{
        CompileInput, CompileOptions, compile_inputs_to_model, compile_inputs_with_catalogue,
    },
    pot::{extract_pot, extract_pot_with_schema},
};
use recite_core::{
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
    schema::ProjectSchema,
};

fn parity(
    inputs: Vec<CompileInput>,
    schema: Option<&ProjectSchema>,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("test")?,
        CompiledAssetId::new("test.recitec")?,
        SourceMapId::new("test.map")?,
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    );
    let model = compile_inputs_to_model(inputs.clone(), options.clone(), schema)?;
    let pot = schema.map_or_else(
        || extract_pot(inputs.clone()),
        |schema| extract_pot_with_schema(inputs.clone(), schema),
    );
    let combined = compile_inputs_with_catalogue(inputs, options, schema)?;
    assert_eq!(combined.diagnostics, model.diagnostics);
    assert_eq!(combined.diagnostics, pot.diagnostics);
    match combined.output {
        Some(output) => {
            assert_eq!(Some(output.dialogue), model.dialogue);
            assert_eq!(Some(output.catalogue), pot.catalog);
        }
        None => {
            assert!(model.dialogue.is_none());
            assert!(pot.catalog.is_none());
        }
    }
    Ok(())
}

#[test]
fn combined_outputs_match_separate_compilation_extraction_and_failures()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [
        include_str!("../../../fixtures/recite/valid/core_language_spike.recite"),
        ":: start default speaker=Mara\n> a@11111111111111111111 bind=(count:int=$remaining)\n  One letter.\n  | {count} letters.\n  ? b@22222222222222222222\n    Yes.\n    -> END\n",
        ":: start default\n> missing_id\n  Text.\n-> END\n",
        ":: start default\n-> missing\n",
        "::\n! effect(\n",
        ":: start default\n> a@11111111111111111111\n  [bad]markup\n",
    ] {
        parity(vec![CompileInput::new("scene.recite", source)], None)?;
    }
    let a = ":: a default\n> a@11111111111111111111\n  Café 🦀.\n-> b.recite::b\n";
    let b = ":: b\n> b@22222222222222222222\n  Goodbye.\n-> END\n";
    parity(
        vec![
            CompileInput::new("b.recite", b),
            CompileInput::new("a.recite", a),
        ],
        None,
    )?;
    // Equal paths must retain caller order in diagnostics and POT references.
    parity(
        vec![
            CompileInput::new("same.recite", a),
            CompileInput::new("same.recite", b),
        ],
        None,
    )?;
    let mut schema = ProjectSchema::empty_v1();
    schema.speakers.insert(
        "Mara".to_owned(),
        recite_core::schema::SpeakerDefinition {
            display_name: Some("Mára".to_owned()),
        },
    );
    parity(
        vec![CompileInput::new(
            "scene.recite",
            ":: start default speaker=Mara\n> hello@11111111111111111111\n  Café 🦀.\n-> END\n",
        )],
        Some(&schema),
    )?;
    parity(
        vec![CompileInput::new(
            "scene.recite",
            ":: start default speaker=missing\n-> END\n",
        )],
        Some(&schema),
    )?;
    Ok(())
}
