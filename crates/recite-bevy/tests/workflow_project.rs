#![cfg(test)]

use std::{fs, path::Path};

use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets, Handle};
use bevy_ecs::message::{MessageCursor, Messages};
use recite_bevy::{
    ReciteConditions, ReciteDialogueAsset, ReciteOutput, ReciteOutputValue, ReciteOwner,
    RecitePlugin, ReciteRequest,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::{
    ChoiceId,
    compiled::{CompiledAssetId, CompilerVersion, SourceMapId},
    schema::load_schema_manifest_str,
};
use recite_runtime::{ConditionValue, DialogueEffectMode, DialogueEvent, EffectAck};

fn asset(edited: bool) -> Result<ReciteDialogueAsset, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/realistic/v1-pack");
    let schema = load_schema_manifest_str(
        "schema/realistic.schema.json",
        &fs::read_to_string(root.join("schema/realistic.schema.json"))?,
    )
    .schema
    .ok_or("fixture schema")?;
    let mut files = fs::read_dir(root.join("src"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    files.sort();
    let inputs = files
        .iter()
        .map(|path| {
            let mut source = fs::read_to_string(path)?;
            if edited {
                source = source.replace("The case narrows", "The case turns");
            }
            Ok(CompileInput::new(
                format!(
                    "src/{}",
                    path.file_name()
                        .ok_or("source filename")?
                        .to_str()
                        .ok_or("UTF-8 source filename")?
                ),
                source,
            ))
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let options = CompileOptions::new(
        CompilerVersion::new("0.1.0")?,
        CompiledAssetId::new("workflow")?,
        SourceMapId::new("workflow.map")?,
        schema.canonical_fingerprint(),
    );
    let report = compile_inputs_with_schema(inputs, options, &schema)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    Ok(ReciteDialogueAsset::from_bytes(
        &report.asset.ok_or("compiled asset")?.messagepack,
    )?)
}

fn send(
    app: &mut App,
    cursor: &mut MessageCursor<ReciteOutput>,
    request: ReciteRequest,
) -> ReciteOutputValue {
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(request);
    app.update();
    cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .last()
        .expect("request output")
        .value
        .clone()
}

fn batch(output: ReciteOutputValue) -> Vec<DialogueEvent> {
    match output {
        ReciteOutputValue::Dialogue(events) => events,
        other => panic!("expected dialogue, got {other:?}"),
    }
}

fn start(handle: Handle<ReciteDialogueAsset>, block: &str) -> ReciteRequest {
    ReciteRequest::Start {
        asset: handle,
        block_id: Some(block.to_owned()),
        locale: None,
        variant: None,
    }
}

#[test]
fn maintained_workflow_preserves_blocking_snapshot_and_active_revision_on_refresh()
-> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    for name in ["object_seen", "trust_at_least", "has_item"] {
        app.world_mut()
            .resource_mut::<ReciteConditions>()
            .register(name, |_, _| Ok(ConditionValue::Bool(true)));
    }
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(asset(false)?);
    app.update();
    let mut cursor = MessageCursor::default();
    let first = batch(send(
        &mut app,
        &mut cursor,
        start(handle.clone(), "arrival"),
    ));
    let pending = first
        .iter()
        .find_map(|event| match event {
            DialogueEvent::Effect(effect) if effect.mode == DialogueEffectMode::Blocking => {
                Some(effect.id.clone())
            }
            _ => None,
        })
        .ok_or("blocking effect")?;
    let save = match send(&mut app, &mut cursor, ReciteRequest::Snapshot) {
        ReciteOutputValue::Snapshot(bytes) => bytes,
        other => panic!("expected snapshot: {other:?}"),
    };
    send(&mut app, &mut cursor, ReciteRequest::End);
    let restored = batch(send(
        &mut app,
        &mut cursor,
        ReciteRequest::Restore {
            asset: handle.clone(),
            snapshot: save,
            variant: None,
        },
    ));
    assert!(
        restored
            .iter()
            .any(|event| matches!(event, DialogueEvent::Effect(effect) if effect.id == pending))
    );
    let prompt = batch(send(
        &mut app,
        &mut cursor,
        ReciteRequest::AcknowledgeEffect {
            effect: pending,
            ack: EffectAck::Completed,
        },
    ));
    assert!(prompt.iter().any(|event| matches!(event, DialogueEvent::Prompt { line: Some(line), choices } if line.source_text.contains("LACUNA") && choices.len() == 3)));
    let original_revision = app
        .world()
        .resource::<ReciteOwner>()
        .active_revision()
        .cloned();
    app.world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .insert(handle.id(), asset(true)?)?;
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<ReciteOwner>().active_revision(),
        original_revision.as_ref()
    );
    let choice = batch(send(
        &mut app,
        &mut cursor,
        ReciteRequest::SelectChoice(ChoiceId::new("21111111111111111111")?),
    ));
    assert!(!choice.is_empty());
    send(&mut app, &mut cursor, ReciteRequest::End);
    let fresh = batch(send(&mut app, &mut cursor, start(handle, "arrival_return")));
    assert!(fresh.iter().any(|event| matches!(event, DialogueEvent::Line(line) if line.source_text.starts_with("The case turns"))));
    assert_ne!(
        app.world().resource::<ReciteOwner>().active_revision(),
        original_revision.as_ref()
    );
    Ok(())
}
