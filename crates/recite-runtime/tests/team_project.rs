#![cfg(test)]

//! Save/restore through the existing runtime API on the multi-author team project fixture.

use std::{fs, path::Path};

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    ChoiceId,
    compiled::{CompiledAssetId, CompiledDialogue, CompilerVersion, SourceMapId},
    schema::load_schema_manifest_str,
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueEffectMode,
    DialogueEffectRequest, DialogueError, DialogueEvent, DialogueSession, EffectAck,
    acknowledge_effect, choose,
    snapshot::{decode_session_messagepack, encode_session_messagepack},
    start_scene,
};

const SCENES: [&str; 3] = ["deck.recite", "dock.recite", "office.recite"];

fn team_asset(edit: impl Fn(&str) -> String) -> CompiledDialogue {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/realistic/team-pack");
    let schema_path = "schema/generated/team.schema.json";
    let schema = load_schema_manifest_str(
        schema_path,
        &fs::read_to_string(root.join(schema_path)).expect("generated schema"),
    )
    .schema
    .expect("valid generated schema");
    let inputs = SCENES.map(|name| {
        let source = fs::read_to_string(root.join("scenes").join(name)).expect("scene source");
        CompileInput::new(format!("scenes/{name}"), edit(&source))
    });
    let report = compile_inputs_with_schema(
        inputs,
        CompileOptions::new(
            CompilerVersion::new("0.1.0").expect("compiler version"),
            CompiledAssetId::new("build/crossing.recitec").expect("asset id"),
            SourceMapId::new("build/crossing.recitec.map").expect("source map id"),
            schema.canonical_fingerprint(),
        ),
        &schema,
    )
    .expect("compile does not hard fail");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    report.asset.expect("compiled asset").dialogue
}

/// The game answers the lantern path: the player carries the lantern and the clerk is friendly.
fn host(query: ConditionQuery<'_>) -> Result<ConditionValue, ConditionEvaluationError> {
    Ok(match query.function() {
        "has_cargo" => ConditionValue::Bool(true),
        "trust_of" => ConditionValue::EnumVariant("friendly".to_owned()),
        other => panic!("fixture host has no answer for {other}"),
    })
}

fn next(asset: &CompiledDialogue, session: &mut DialogueSession) -> DialogueEvent {
    recite_runtime::next(asset, session, &host).expect("traversal succeeds")
}

fn save_and_load(asset: &CompiledDialogue, session: &DialogueSession) -> DialogueSession {
    let bytes = encode_session_messagepack(session).expect("session encodes");
    decode_session_messagepack(asset, &bytes).expect("session restores on the same asset")
}

fn blocking_effect(event: DialogueEvent) -> DialogueEffectRequest {
    match event {
        DialogueEvent::Effect(effect) if effect.mode == DialogueEffectMode::Blocking => effect,
        other => panic!("expected a blocking effect, got {other:?}"),
    }
}

fn line_id(event: DialogueEvent) -> String {
    match event {
        DialogueEvent::Line(line) => line.id.as_str().to_owned(),
        other => panic!("expected a line, got {other:?}"),
    }
}

#[test]
fn team_project_restores_prompt_and_blocking_effect_and_rejects_a_changed_asset() {
    let asset = team_asset(str::to_owned);
    let mut session = start_scene(&asset, Some("dock")).expect("dock starts");
    assert!(matches!(
        next(&asset, &mut session),
        DialogueEvent::Effect(effect) if effect.function == "ring_bell"
    ));
    let DialogueEvent::Prompt { choices, .. } = next(&asset, &mut session) else {
        panic!("expected the dock prompt");
    };
    let offered: Vec<_> = choices.iter().map(|choice| choice.id.as_str()).collect();
    assert_eq!(offered, ["dock_lantern", "dock_ledger"]);

    // Save at the prompt; the restored session accepts the same stable choice ID.
    let mut at_prompt = save_and_load(&asset, &session);
    let chosen = choose(
        &asset,
        &mut at_prompt,
        ChoiceId::new("dock_lantern").expect("choice ID"),
        &host,
    )
    .expect("restored prompt accepts the choice");
    assert_eq!(line_id(chosen), "deck_crossing");

    // Save at the blocking effect; restoring re-emits the same request ID until it is answered.
    let effect = blocking_effect(next(&asset, &mut at_prompt));
    assert_eq!(effect.function, "load_cargo");
    let saved = encode_session_messagepack(&at_prompt).expect("blocked session encodes");
    let mut restored = decode_session_messagepack(&asset, &saved).expect("blocked session loads");
    let reemitted = blocking_effect(next(&asset, &mut restored));
    assert_eq!(reemitted.id, effect.id);
    acknowledge_effect(&mut restored, reemitted.id, EffectAck::Completed)
        .expect("host acknowledges the original request");
    assert_eq!(line_id(next(&asset, &mut restored)), "arrival_line");
    assert!(matches!(
        next(&asset, &mut restored),
        DialogueEvent::End { .. }
    ));

    // Another author's merged wording change produces a different asset; the old save is refused.
    let edited = team_asset(|source| source.replace("Keep the lantern low.", "Keep it low."));
    assert!(matches!(
        decode_session_messagepack(&edited, &saved),
        Err(DialogueError::AssetContentMismatch { .. })
    ));
}
