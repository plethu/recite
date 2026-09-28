use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets, Handle};
use bevy_ecs::message::{MessageCursor, Messages};
use recite_bevy::{
    ReciteAssetImport, ReciteAssetStatus, ReciteCatalog, ReciteConditions, ReciteDialogueAsset,
    ReciteOutput, ReciteOutputValue, ReciteOwner, RecitePlugin, ReciteRequest,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, LocaleId,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};
use recite_runtime::{
    ConditionValue, DialogueEffectMode, DialogueEvent, EffectAck, localisation::TextDomain,
};

fn compiled(source: &str, asset_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let options = CompileOptions::new(
        CompilerVersion::new("0.1.0")?,
        CompiledAssetId::new(asset_id)?,
        SourceMapId::new("test-source-map")?,
        SchemaFingerprint::NoSchema,
    );
    let report = compile_inputs([CompileInput::new("test.recite", source)], options)?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    Ok(report
        .asset
        .ok_or_else(|| std::io::Error::other("missing compiled fixture"))?
        .messagepack)
}

#[test]
fn accepted_refresh_only_affects_next_session() -> Result<(), Box<dyn std::error::Error>> {
    let old = ":: start default\n? go@11111111111111111111\n  Go.\n  -> later\n:: later\n> result@22222222222222222222\n  Old text.\n-> END\n";
    let new = old.replace("Old text.", "New text.");
    let mut app = app();
    let handle = add_asset(&mut app, &compiled(old, "dialogue/refresh.recitec")?)?;
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    let mut statuses = MessageCursor::<ReciteAssetStatus>::default();

    request(
        &mut app,
        ReciteRequest::Start {
            asset: handle.clone(),
            block_id: None,
            locale: None,
            variant: None,
        },
    );
    outputs(&app, &mut cursor);
    let active_before = app
        .world()
        .resource::<ReciteOwner>()
        .active_revision()
        .cloned()
        .expect("active revision");
    let candidate = ReciteDialogueAsset::from_bytes(&compiled(&new, "dialogue/refresh.recitec")?)?;
    let candidate_revision = candidate.revision().clone();
    app.world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .insert(handle.id(), candidate)
        .expect("replace asset");
    app.update();
    app.update();
    let observed = statuses
        .read(app.world().resource::<Messages<ReciteAssetStatus>>())
        .cloned()
        .collect::<Vec<_>>();
    assert!(observed.iter().any(|status| matches!(&status.import,
        ReciteAssetImport::Accepted { revision } if revision == &candidate_revision)));
    assert_ne!(active_before.fingerprint, candidate_revision.fingerprint);
    assert_eq!(
        app.world().resource::<ReciteOwner>().active_revision(),
        Some(&active_before)
    );

    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("11111111111111111111").expect("choice")),
    );
    let active = outputs(&app, &mut cursor);
    assert!(
        matches!(&active[0].value, ReciteOutputValue::Dialogue(events) if events.iter().any(|event| matches!(event, DialogueEvent::Line(line) if line.text == "Old text.")))
    );
    request(&mut app, ReciteRequest::End);
    outputs(&app, &mut cursor);
    request(
        &mut app,
        ReciteRequest::Start {
            asset: handle,
            block_id: None,
            locale: None,
            variant: None,
        },
    );
    outputs(&app, &mut cursor);
    assert_eq!(
        app.world().resource::<ReciteOwner>().active_revision(),
        Some(&candidate_revision)
    );
    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("11111111111111111111").expect("choice")),
    );
    let next = outputs(&app, &mut cursor);
    assert!(
        matches!(&next[0].value, ReciteOutputValue::Dialogue(events) if events.iter().any(|event| matches!(event, DialogueEvent::Line(line) if line.text == "New text.")))
    );
    Ok(())
}

#[test]
fn app_handles_conditions_effects_locale_and_snapshot_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/recite/valid/adapter_conformance/runtime_surface.recite"
    ))
    .expect("published conformance source");
    let mut app = app();
    app.world_mut()
        .resource_mut::<ReciteConditions>()
        .register("trusts", |_, _| Ok(ConditionValue::Bool(false)));
    app.world_mut()
        .resource_mut::<ReciteCatalog>()
        .0
        .insert_for_domain(
            "fr-FR",
            TextDomain::Line,
            "e0053a232c1bc1128c47",
            "Intro line.",
            "Bonjour.",
            Some("formal"),
        )
        .expect("catalog entry");
    let handle = add_asset(
        &mut app,
        &compiled(&source, "dialogue/runtime_surface.recitec")?,
    )?;
    let mut cursor = MessageCursor::<ReciteOutput>::default();

    request(
        &mut app,
        ReciteRequest::Start {
            asset: handle.clone(),
            block_id: None,
            locale: Some(LocaleId::new("fr-FR").expect("locale")),
            variant: Some("formal".to_owned()),
        },
    );
    let start = outputs(&app, &mut cursor);
    let ReciteOutputValue::Dialogue(events) = &start[0].value else {
        panic!("start dialogue");
    };
    assert!(
        matches!(&events[0], DialogueEvent::Line(line) if line.text == "Bonjour." && line.source_text == "Intro line.")
    );
    assert!(
        matches!(&events[1], DialogueEvent::Effect(effect) if effect.mode == DialogueEffectMode::Immediate && effect.function == "intro_sfx")
    );
    assert!(
        matches!(&events[2], DialogueEvent::Prompt { choices, .. } if choices.len() == 2 && !choices[1].availability.is_available)
    );

    request(&mut app, ReciteRequest::Snapshot);
    let prompt_snapshot = match &outputs(&app, &mut cursor)[0].value {
        ReciteOutputValue::Snapshot(bytes) => bytes.clone(),
        other => panic!("prompt snapshot, got {other:?}"),
    };
    request(&mut app, ReciteRequest::End);
    outputs(&app, &mut cursor);
    request(
        &mut app,
        ReciteRequest::Restore {
            asset: handle.clone(),
            snapshot: prompt_snapshot,
            variant: Some("formal".to_owned()),
        },
    );
    outputs(&app, &mut cursor);
    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("3f481d9991fa22e23b0c").expect("work choice")),
    );
    let work = outputs(&app, &mut cursor);
    let ReciteOutputValue::Dialogue(events) = &work[0].value else {
        panic!("work dialogue");
    };
    let effect_id = match events.last() {
        Some(DialogueEvent::Effect(effect))
            if effect.mode == DialogueEffectMode::Blocking && effect.function == "grant_item" =>
        {
            effect.id.clone()
        }
        other => panic!("blocking effect, got {other:?}"),
    };

    request(&mut app, ReciteRequest::Snapshot);
    let blocking_snapshot = match &outputs(&app, &mut cursor)[0].value {
        ReciteOutputValue::Snapshot(bytes) => bytes.clone(),
        other => panic!("blocking snapshot, got {other:?}"),
    };
    request(&mut app, ReciteRequest::End);
    outputs(&app, &mut cursor);
    request(
        &mut app,
        ReciteRequest::Restore {
            asset: handle,
            snapshot: blocking_snapshot,
            variant: Some("formal".to_owned()),
        },
    );
    outputs(&app, &mut cursor);
    request(
        &mut app,
        ReciteRequest::AcknowledgeEffect {
            effect: effect_id,
            ack: EffectAck::Completed,
        },
    );
    let resumed = outputs(&app, &mut cursor);
    assert!(
        matches!(&resumed[0].value, ReciteOutputValue::Dialogue(events) if events.iter().any(|event| matches!(event, DialogueEvent::Prompt { .. })))
    );
    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("ced14b8623bba2198c6d").expect("end choice")),
    );
    let ending = outputs(&app, &mut cursor);
    assert!(
        matches!(&ending[0].value, ReciteOutputValue::Dialogue(events) if events.iter().any(|event| matches!(event, DialogueEvent::End { deferred_effects } if deferred_effects.iter().any(|effect| effect.mode == DialogueEffectMode::Deferred && effect.function == "finished_work"))))
    );
    Ok(())
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((AssetPlugin::default(), RecitePlugin));
    app
}

fn add_asset(
    app: &mut App,
    bytes: &[u8],
) -> Result<Handle<ReciteDialogueAsset>, Box<dyn std::error::Error>> {
    let asset = ReciteDialogueAsset::from_bytes(bytes)?;
    let handle = app
        .world_mut()
        .resource_mut::<Assets<ReciteDialogueAsset>>()
        .add(asset);
    app.update();
    Ok(handle)
}

fn request(app: &mut App, request: ReciteRequest) {
    app.world_mut()
        .resource_mut::<Messages<ReciteRequest>>()
        .write(request);
    app.update();
}

fn outputs(app: &App, cursor: &mut MessageCursor<ReciteOutput>) -> Vec<ReciteOutput> {
    cursor
        .read(app.world().resource::<Messages<ReciteOutput>>())
        .cloned()
        .collect()
}

#[test]
fn app_delivers_ordered_dialogue_and_stable_choice() -> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> intro@11111111111111111111\n  Hello.\n? yes@22222222222222222222\n  Yes.\n  -> END\n";
    let mut app = app();
    let handle = add_asset(&mut app, &compiled(source, "dialogue/first.recitec")?)?;
    let mut cursor = MessageCursor::<ReciteOutput>::default();

    request(
        &mut app,
        ReciteRequest::Start {
            asset: handle,
            block_id: None,
            locale: None,
            variant: None,
        },
    );
    let first = outputs(&app, &mut cursor);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].sequence, 0);
    let ReciteOutputValue::Dialogue(events) = &first[0].value else {
        panic!("start should emit dialogue");
    };
    assert!(matches!(&events[0], DialogueEvent::Line(line) if line.text == "Hello."));
    assert!(
        matches!(&events[1], DialogueEvent::Prompt { choices, .. } if choices[0].id.as_str() == "22222222222222222222")
    );

    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("22222222222222222222").expect("choice ID")),
    );
    let selected = outputs(&app, &mut cursor);
    assert_eq!(selected[0].sequence, 1);
    assert!(
        matches!(&selected[0].value, ReciteOutputValue::Dialogue(events) if events.iter().any(|event| matches!(event, DialogueEvent::End { .. })))
    );
    assert!(app.world().resource::<ReciteOwner>().has_active_session());
    request(&mut app, ReciteRequest::End);
    assert!(matches!(
        outputs(&app, &mut cursor)[0].value,
        ReciteOutputValue::Ended
    ));
    assert!(!app.world().resource::<ReciteOwner>().has_active_session());
    Ok(())
}

#[test]
fn active_session_borrows_replaced_catalogue_on_next_operation()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> opening@11111111111111111111\n  Hello.\n? proceed@22222222222222222222\n  Continue.\n  -> later\n:: later\n> closing@33333333333333333333\n  Goodbye.\n-> END\n";
    let mut app = app();
    let handle = add_asset(&mut app, &compiled(source, "dialogue/catalog.recitec")?)?;
    let mut old = ReciteCatalog::default();
    old.0
        .insert_for_domain(
            "fr-FR",
            TextDomain::Line,
            "11111111111111111111",
            "Hello.",
            "Bonjour.",
            None,
        )
        .expect("old opening");
    old.0
        .insert_for_domain(
            "fr-FR",
            TextDomain::Line,
            "33333333333333333333",
            "Goodbye.",
            "Au revoir.",
            None,
        )
        .expect("old closing");
    app.insert_resource(old);
    let mut cursor = MessageCursor::<ReciteOutput>::default();
    request(
        &mut app,
        ReciteRequest::Start {
            asset: handle,
            block_id: None,
            locale: Some(LocaleId::new("fr-FR").expect("locale")),
            variant: None,
        },
    );
    let first = outputs(&app, &mut cursor);
    assert!(
        matches!(&first[0].value, ReciteOutputValue::Dialogue(events)
        if matches!(&events[0], DialogueEvent::Line(line) if line.text == "Bonjour."))
    );

    let mut new = ReciteCatalog::default();
    new.0
        .import_po(
            "fr-FR",
            "fr-FR.po",
            "msgctxt \"33333333333333333333\"\nmsgid \"Goodbye.\"\nmsgstr \"Adieu.\"\n",
        )
        .expect("canonical PO import");
    let malformed = new
        .0
        .import_po("fr-FR", "broken.po", "msgid \"Broken\"\nmsgstr \"Cassé\"\n");
    assert!(malformed.is_err(), "failed PO import is transactional");
    app.insert_resource(new);
    request(
        &mut app,
        ReciteRequest::SelectChoice(ChoiceId::new("22222222222222222222").expect("choice")),
    );
    let resumed = outputs(&app, &mut cursor);
    assert!(
        matches!(&resumed[0].value, ReciteOutputValue::Dialogue(events)
        if events.iter().any(|event| matches!(event, DialogueEvent::Line(line) if line.text == "Adieu.")))
    );
    Ok(())
}
