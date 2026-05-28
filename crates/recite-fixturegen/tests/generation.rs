use std::collections::BTreeMap;

use recite_compiler::{
    CompileInput, CompileOptions, compile_inputs_with_schema, extract_pot_with_schema,
};
use recite_core::{
    CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId, load_schema_manifest_str,
};
use recite_fixturegen::{FixtureConfigSet, FixtureProfile, SummarySet, generate_tiny_in_memory};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueContext, DialogueEvent,
    EffectAck, acknowledge_effect, choose, next, start_scene,
};

fn tiny_profile(seed: u64) -> FixtureProfile {
    FixtureProfile {
        name: "tiny".to_owned(),
        seed,
        blocks: 10,
        lines: 100,
        choices: 20,
        localisable_entries: 120,
        generated_words: 1_000,
        shards: 2,
    }
}

#[test]
fn same_profile_seed_produces_byte_identical_tiny_output() {
    let first = generate_tiny_in_memory(&tiny_profile(72)).expect("first fixture");
    let second = generate_tiny_in_memory(&tiny_profile(72)).expect("second fixture");

    assert_eq!(first.files, second.files);
    assert_eq!(first.summary, second.summary);
}

#[test]
fn changed_seed_changes_summary_hash() {
    let first = generate_tiny_in_memory(&tiny_profile(72)).expect("first fixture");
    let second = generate_tiny_in_memory(&tiny_profile(73)).expect("second fixture");

    assert_ne!(first.summary.summary_hash, second.summary.summary_hash);
}

#[test]
fn profile_counts_match_spec_budgets() {
    let profiles_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/synthetic/profiles.toml");
    let profiles = FixtureConfigSet::load_path(&profiles_path).expect("profiles load");
    for expected in [
        ("tiny", 10, 100, 20, 120, 1_000),
        ("small", 100, 1_000, 200, 1_200, 10_000),
        ("medium", 1_000, 10_000, 2_000, 12_000, 100_000),
        ("large", 5_000, 50_000, 10_000, 60_000, 500_000),
        ("epic", 10_000, 80_000, 20_000, 100_000, 1_000_000),
    ] {
        let summary = SummarySet::generate_one(profiles.profile(expected.0).expect("profile"))
            .expect("summary");
        assert_eq!(summary.counts.blocks, expected.1);
        assert_eq!(summary.counts.lines, expected.2);
        assert_eq!(summary.counts.choices, expected.3);
        assert_eq!(summary.counts.localisable_entries, expected.4);
        assert!(summary.counts.generated_words >= expected.5);
    }
}

#[test]
fn generated_tiny_fixture_validates_compiles_extracts_and_traverses() {
    let generated = generate_tiny_in_memory(&tiny_profile(72)).expect("tiny fixture");
    let schema_source = text(&generated.files, "schema/synthetic.schema.json");
    let schema = load_schema_manifest_str("schema/synthetic.schema.json", schema_source)
        .schema
        .expect("schema loads");
    let inputs = source_inputs(&generated.files);
    let options = CompileOptions::new(
        CompilerVersion::new("fixturegen-test").expect("compiler version"),
        CompiledAssetId::new("synthetic-tiny").expect("asset id"),
        SourceMapId::new("synthetic-tiny-map").expect("source map id"),
        SchemaFingerprint::NoSchema,
    );
    let report =
        compile_inputs_with_schema(inputs.clone(), options, &schema).expect("compile succeeds");
    assert!(
        report.diagnostics.is_empty(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let asset = report.asset.expect("compiled asset").dialogue;

    let pot = extract_pot_with_schema(inputs, &schema);
    assert!(
        pot.diagnostics.is_empty(),
        "pot diagnostics: {:?}",
        pot.diagnostics
    );
    assert_eq!(
        pot.catalog.expect("catalog").entries.len(),
        generated.summary.counts.localisable_entries as usize + 9
    );

    let mut session = start_scene(&asset, Some("block_00000")).expect("start scene");
    let context = FixtureContext;
    let mut saw_prompt = false;
    let mut saw_blocking = false;
    for _ in 0..80 {
        let event = next(&asset, &mut session, &context).expect("next");
        let should_stop = handle_event(
            event,
            &asset,
            &mut session,
            &context,
            &mut saw_prompt,
            &mut saw_blocking,
        );
        if should_stop {
            break;
        }
    }

    assert!(saw_prompt, "runtime traversal should reach a prompt");
    assert!(
        saw_blocking,
        "runtime traversal should emit a blocking effect"
    );
}

fn handle_event(
    event: DialogueEvent,
    asset: &recite_core::CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    context: &dyn DialogueContext,
    saw_prompt: &mut bool,
    saw_blocking: &mut bool,
) -> bool {
    match event {
        DialogueEvent::Effect(effect) => {
            if matches!(effect.mode, recite_runtime::DialogueEffectMode::Blocking) {
                *saw_blocking = true;
                acknowledge_effect(session, effect.id, EffectAck::Completed).expect("ack");
            }
            false
        }
        DialogueEvent::Prompt { choices, .. } => {
            *saw_prompt = true;
            let selected = choices[0].id.clone();
            let event = choose(asset, session, selected, context).expect("choose");
            match event {
                DialogueEvent::End { .. } => true,
                nested => handle_event(nested, asset, session, context, saw_prompt, saw_blocking),
            }
        }
        DialogueEvent::End { .. } => true,
        DialogueEvent::Line(_) => false,
    }
}

struct FixtureContext;

impl DialogueContext for FixtureContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        match query.function() {
            "flag" | "counter_gte" => Ok(ConditionValue::Bool(true)),
            "relationship" => Ok(ConditionValue::EnumVariant("active".to_owned())),
            function => Err(ConditionEvaluationError::new(format!(
                "unexpected condition `{function}`"
            ))),
        }
    }
}
fn source_inputs(files: &BTreeMap<String, Vec<u8>>) -> Vec<CompileInput> {
    files
        .iter()
        .filter_map(|(path, bytes)| {
            path.strip_prefix("src/").map(|_| {
                CompileInput::new(
                    path.clone(),
                    String::from_utf8(bytes.clone()).expect("utf8"),
                )
            })
        })
        .collect()
}

fn text<'a>(files: &'a BTreeMap<String, Vec<u8>>, path: &str) -> &'a str {
    std::str::from_utf8(files.get(path).expect("file exists")).expect("utf8")
}
