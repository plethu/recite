//! Informational CPU-only probe. Build once, then time individual modes with
//! an external process timer; no timing API enters the library.

use std::error::Error;
use std::hint::black_box;
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets};
use bevy_ecs::message::{MessageCursor, Messages};
use recite_bevy::{
    ReciteConditions, ReciteDialogueAsset, ReciteOutput, ReciteOutputValue, ReciteOwner,
    RecitePlugin, ReciteRequest,
};
use recite_core::ChoiceId;
use recite_runtime::{ConditionValue, DialogueEvent, EffectAck};

fn send(
    app: &mut App,
    cursor: &mut MessageCursor<ReciteOutput>,
    request: ReciteRequest,
) -> Result<ReciteOutput, io::Error> {
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(request);
    app.update();
    cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .last()
        .cloned()
        .ok_or_else(|| io::Error::other("missing adapter output"))
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    app
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args
        .next()
        .ok_or_else(|| io::Error::other("expected mode"))?;
    let source = args
        .next()
        .ok_or_else(|| io::Error::other("expected compiled asset"))?;
    let count: usize = args
        .next()
        .ok_or_else(|| io::Error::other("expected iteration count"))?
        .parse()?;
    let bytes = std::fs::read(source)?;
    match mode.as_str() {
        "load" => {
            for _ in 0..count {
                black_box(ReciteDialogueAsset::from_bytes(&bytes)?.revision().clone());
            }
            println!("load_conversions={count} bytes={}", bytes.len());
        }
        "idle" => {
            let mut app = app();
            for _ in 0..count {
                app.update();
            }
            println!("idle_updates={count} active_sessions=0");
        }
        "active" => {
            let mut app = app();
            let checks = Arc::new(AtomicU64::new(0));
            let observed = Arc::clone(&checks);
            app.world_mut()
                .resource_mut::<ReciteConditions>()
                .register("trusts", move |_, _| {
                    observed.fetch_add(1, Ordering::Relaxed);
                    Ok(ConditionValue::Bool(false))
                });
            let handle = app
                .world_mut()
                .resource_mut::<Assets<ReciteDialogueAsset>>()
                .add(ReciteDialogueAsset::from_bytes(&bytes)?);
            app.update();
            let mut cursor = MessageCursor::<ReciteOutput>::default();
            for _ in 0..count {
                let first = send(
                    &mut app,
                    &mut cursor,
                    ReciteRequest::Start {
                        asset: handle.clone(),
                        block_id: None,
                        locale: None,
                        variant: None,
                    },
                )?;
                assert!(matches!(first.value, ReciteOutputValue::Dialogue(_)));
                let work = send(
                    &mut app,
                    &mut cursor,
                    ReciteRequest::SelectChoice(ChoiceId::new("3f481d9991fa22e23b0c")?),
                )?;
                let ReciteOutputValue::Dialogue(events) = work.value else {
                    return Err(io::Error::other("missing work dialogue").into());
                };
                let effect = events
                    .iter()
                    .find_map(|event| match event {
                        DialogueEvent::Effect(effect) => Some(effect.id.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| io::Error::other("missing blocking effect"))?;
                let resumed = send(
                    &mut app,
                    &mut cursor,
                    ReciteRequest::AcknowledgeEffect {
                        effect,
                        ack: EffectAck::Completed,
                    },
                )?;
                assert!(matches!(resumed.value, ReciteOutputValue::Dialogue(_)));
                let ending = send(
                    &mut app,
                    &mut cursor,
                    ReciteRequest::SelectChoice(ChoiceId::new("ced14b8623bba2198c6d")?),
                )?;
                assert!(matches!(ending.value, ReciteOutputValue::Dialogue(_)));
                let ended = send(&mut app, &mut cursor, ReciteRequest::End)?;
                assert!(matches!(ended.value, ReciteOutputValue::Ended));
            }
            println!(
                "active_cycles={count} condition_dispatches={}",
                checks.load(Ordering::Relaxed)
            );
        }
        "retained" => {
            let changed = args
                .next()
                .ok_or_else(|| io::Error::other("expected changed asset"))?;
            let mut app = app();
            app.world_mut()
                .resource_mut::<ReciteConditions>()
                .register("trusts", |_, _| Ok(ConditionValue::Bool(false)));
            let handle = app
                .world_mut()
                .resource_mut::<Assets<ReciteDialogueAsset>>()
                .add(ReciteDialogueAsset::from_bytes(&bytes)?);
            app.update();
            let mut cursor = MessageCursor::<ReciteOutput>::default();
            send(
                &mut app,
                &mut cursor,
                ReciteRequest::Start {
                    asset: handle.clone(),
                    block_id: None,
                    locale: None,
                    variant: None,
                },
            )?;
            let old = app
                .world()
                .resource::<ReciteOwner>()
                .active_revision()
                .cloned()
                .ok_or_else(|| io::Error::other("missing active revision"))?;
            let candidate = ReciteDialogueAsset::from_bytes(&std::fs::read(&changed)?)?;
            let new = candidate.revision().clone();
            app.world_mut()
                .resource_mut::<Assets<ReciteDialogueAsset>>()
                .insert(handle.id(), candidate)?;
            app.update();
            app.update();
            assert_ne!(old.fingerprint, new.fingerprint);
            assert_eq!(
                app.world().resource::<ReciteOwner>().active_revision(),
                Some(&old)
            );
            send(&mut app, &mut cursor, ReciteRequest::End)?;
            send(
                &mut app,
                &mut cursor,
                ReciteRequest::Start {
                    asset: handle,
                    block_id: None,
                    locale: None,
                    variant: None,
                },
            )?;
            assert_eq!(
                app.world().resource::<ReciteOwner>().active_revision(),
                Some(&new)
            );
            println!(
                "observed_revision_identities_during_active_refresh=2 bytes_old={} bytes_new={}",
                bytes.len(),
                std::fs::metadata(changed)?.len()
            );
        }
        _ => return Err(io::Error::other("unknown probe mode").into()),
    }
    Ok(())
}
