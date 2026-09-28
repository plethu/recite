//! Run with `cargo run -p recite-bevy --example headless_dialogue`.
//! Compilation here is authoring-only: `recite-compiler` is a dev dependency.

use std::io;

use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets, Handle};
use bevy_ecs::{
    message::{MessageCursor, Messages},
    resource::Resource,
};
use recite_bevy::{
    ReciteCatalog, ReciteConditions, ReciteDialogueAsset, ReciteOutput, ReciteOutputValue,
    RecitePlugin, ReciteRequest, ReciteSchema,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    ChoiceId, LocaleId,
    compiled::{CompiledAssetId, CompilerVersion, SourceMapId},
    schema::{ProducerIdentity, load_schema_manifest_str},
};
use recite_runtime::{
    ConditionValue, DialogueEffectMode, DialogueEffectRequest, DialogueEvent, EffectAck,
    localisation::TextDomain,
};

#[derive(Resource)]
struct Clearance(bool);

fn schema() -> Result<ReciteSchema, recite_bevy::SchemaRegistrationError> {
    let mut schema = ReciteSchema::new();
    schema.condition("has_clearance")?.returns_bool();
    let _ = schema.effect("play_chime")?.immediate();
    let _ = schema.effect("grant_key")?.blocking();
    let _ = schema.effect("mark_seen")?.deferred();
    Ok(schema)
}

fn compile(
    source: &str,
    schema: &ReciteSchema,
) -> Result<ReciteDialogueAsset, Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("0.1.0")?,
        CompiledAssetId::new("dialogue/demo.recitec")?,
        SourceMapId::new("demo-source-map")?,
        schema.schema().canonical_fingerprint(),
    );
    let report = compile_inputs_with_schema(
        [CompileInput::new("dialogue/demo.recite", source)],
        options,
        schema.schema(),
    )?;
    if !report.diagnostics.is_empty() {
        return Err(io::Error::other(format!(
            "schema or dialogue diagnostics: {:?}",
            report.diagnostics
        ))
        .into());
    }
    let bytes = report
        .asset
        .ok_or_else(|| io::Error::other("no compiled dialogue"))?
        .messagepack;
    Ok(ReciteDialogueAsset::from_bytes(&bytes)?)
}

fn send(
    app: &mut App,
    cursor: &mut MessageCursor<ReciteOutput>,
    request: ReciteRequest,
) -> Vec<ReciteOutput> {
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(request);
    app.update();
    cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .cloned()
        .collect()
}

fn batch(outputs: &[ReciteOutput]) -> Result<&[DialogueEvent], io::Error> {
    match outputs.first().map(|output| &output.value) {
        Some(ReciteOutputValue::Dialogue(events)) => Ok(events),
        Some(ReciteOutputValue::Error { error, .. }) => Err(io::Error::other(error.clone())),
        _ => Err(io::Error::other("expected dialogue output")),
    }
}

fn snapshot(outputs: &[ReciteOutput]) -> Result<Vec<u8>, io::Error> {
    match outputs.first().map(|output| &output.value) {
        Some(ReciteOutputValue::Snapshot(bytes)) => Ok(bytes.clone()),
        _ => Err(io::Error::other("expected session snapshot")),
    }
}

fn game_effect(request: &DialogueEffectRequest) -> Result<&'static str, io::Error> {
    match (request.function.as_str(), request.mode) {
        ("play_chime", DialogueEffectMode::Immediate) => Ok("play_chime"),
        ("grant_key", DialogueEffectMode::Blocking) => Ok("grant_key"),
        ("mark_seen", DialogueEffectMode::Deferred) => Ok("mark_seen"),
        _ => Err(io::Error::other("unexpected typed game effect")),
    }
}

fn start(handle: Handle<ReciteDialogueAsset>) -> Result<ReciteRequest, Box<dyn std::error::Error>> {
    Ok(ReciteRequest::Start {
        asset: handle,
        block_id: None,
        locale: Some(LocaleId::new("fr-FR")?),
        variant: Some("formal".to_owned()),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema()?;
    let generated_manifest = schema
        .export_json_with_producer(ProducerIdentity::new("bevy", "recite-demo/schema")?)
        .map_err(|diagnostics| io::Error::other(format!("invalid host schema: {diagnostics:?}")))?;
    assert!(generated_manifest.contains("has_clearance"));
    let generated = load_schema_manifest_str("demo-schema.json", &generated_manifest);
    let metadata = generated
        .schema
        .ok_or_else(|| io::Error::other("generated schema failed canonical reload"))?
        .producer_metadata
        .ok_or_else(|| io::Error::other("generated schema lacks producer metadata"))?;
    assert_eq!(
        metadata
            .producer
            .ok_or_else(|| io::Error::other("generated schema lacks producer"))?
            .kind(),
        "bevy"
    );
    assert_eq!(metadata.producer_fingerprints.len(), 1);

    let source = include_str!("assets/demo.recite");
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    app.insert_resource(Clearance(true));
    app.world_mut()
        .resource_mut::<ReciteConditions>()
        .register("has_clearance", |world, _| {
            Ok(ConditionValue::Bool(world.resource::<Clearance>().0))
        });
    app.world_mut()
        .resource_mut::<ReciteCatalog>()
        .0
        .insert_for_domain(
            "fr-FR",
            TextDomain::Line,
            "11111111111111111111",
            "Welcome.",
            "Bienvenue.",
            Some("formal"),
        )?;
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(compile(source, &schema)?);
    app.update();
    let mut cursor = MessageCursor::<ReciteOutput>::default();

    let opening = send(&mut app, &mut cursor, start(handle.clone())?);
    let first = batch(&opening)?;
    assert!(matches!(&first[0], DialogueEvent::Line(line) if line.text == "Bienvenue."));
    if let DialogueEvent::Effect(effect) = &first[1] {
        println!("game handles {} ({:?})", game_effect(effect)?, effect.id);
    }
    let prompt_save = snapshot(&send(&mut app, &mut cursor, ReciteRequest::Snapshot))?;
    send(&mut app, &mut cursor, ReciteRequest::End);
    batch(&send(
        &mut app,
        &mut cursor,
        ReciteRequest::Restore {
            asset: handle.clone(),
            snapshot: prompt_save,
            variant: Some("formal".to_owned()),
        },
    ))?;

    let work = send(
        &mut app,
        &mut cursor,
        ReciteRequest::SelectChoice(ChoiceId::new("22222222222222222222")?),
    );
    let pending = batch(&work)?
        .iter()
        .find_map(|event| match event {
            DialogueEvent::Effect(effect) if effect.mode == DialogueEffectMode::Blocking => {
                Some(effect)
            }
            _ => None,
        })
        .ok_or_else(|| io::Error::other("expected blocking effect"))?;
    println!("game handles {} ({:?})", game_effect(pending)?, pending.id);
    let effect_id = pending.id.clone();
    let blocked_save = snapshot(&send(&mut app, &mut cursor, ReciteRequest::Snapshot))?;
    send(&mut app, &mut cursor, ReciteRequest::End);
    batch(&send(
        &mut app,
        &mut cursor,
        ReciteRequest::Restore {
            asset: handle.clone(),
            snapshot: blocked_save,
            variant: Some("formal".to_owned()),
        },
    ))?;
    let ending = send(
        &mut app,
        &mut cursor,
        ReciteRequest::AcknowledgeEffect {
            effect: effect_id,
            ack: EffectAck::Completed,
        },
    );
    for event in batch(&ending)? {
        if let DialogueEvent::End { deferred_effects } = event {
            for effect in deferred_effects {
                println!("game handles {} ({:?})", game_effect(effect)?, effect.id);
            }
        }
    }

    // The authoring loop rebuilds changed source, then Bevy accepts that new
    // revision for the next session. Active sessions retain their prior bytes.
    let changed = source.replace("Welcome.", "Welcome back.");
    let refreshed = compile(&changed, &schema)?;
    app.world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .insert(handle.id(), refreshed)?;
    app.update();
    app.update();
    send(&mut app, &mut cursor, ReciteRequest::End);
    let next = send(&mut app, &mut cursor, start(handle)?);
    assert!(
        matches!(&batch(&next)?[0], DialogueEvent::Line(line) if line.source_text == "Welcome back.")
    );
    println!("next session uses rebuilt compiled revision");
    Ok(())
}
