use recite_adapter::{
    AdapterErrorKind, DriverError, LoadedDialogue, ReciteDialogueCatalog, SessionDriver,
    StartRequest,
};
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, LocaleId,
    compiled::{CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId},
};
use recite_runtime::{
    DialogueEvent, DialogueSessionOptions, EmptyDialogueContext, LocaleResolution,
};

fn asset(source: &str) -> Result<LoadedDialogue, Box<dyn std::error::Error>> {
    let report = compile_inputs(
        [CompileInput::new("test.recite", source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")?,
            CompiledAssetId::new("test.recitec")?,
            SourceMapId::new("test.recitec.map")?,
            SchemaFingerprint::NoSchema,
        ),
    )?;
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let compiled = report
        .asset
        .ok_or_else(|| std::io::Error::other("compiler emitted no asset"))?;
    Ok(LoadedDialogue::from_bytes(&compiled.messagepack)?)
}

fn request(asset: &LoadedDialogue) -> StartRequest<'_> {
    StartRequest {
        asset,
        block_id: None,
        options: DialogueSessionOptions::new(),
    }
}

#[test]
fn missing_catalogue_entry_delivers_authored_source_text() -> Result<(), Box<dyn std::error::Error>>
{
    let asset =
        asset(":: start default\n> intro@11111111111111111111\n  Authored source.\n-> END\n")?;
    let catalogue = ReciteDialogueCatalog::new();
    let mut driver = SessionDriver::new();
    let events = driver.start(
        StartRequest {
            asset: &asset,
            block_id: None,
            options: DialogueSessionOptions::new().with_locale(LocaleId::new("fr-CA")?),
        },
        &EmptyDialogueContext,
        LocaleResolution::new().with_provider(&catalogue),
    )?;
    assert!(matches!(&events[0], DialogueEvent::Line(line) if line.text == "Authored source."));
    Ok(())
}

#[test]
fn prepared_begin_is_single_use_and_failed_encoding_rolls_back()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = asset(":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n")?;
    let mut driver = SessionDriver::new();
    driver.prepare(request(&asset))?;
    assert!(driver.is_prepared());
    let failure = driver.begin_with(&EmptyDialogueContext, LocaleResolution::new(), |_| {
        Err::<(), _>("encoding failed")
    });
    assert!(matches!(
        failure,
        Err(DriverError::Output("encoding failed"))
    ));
    assert!(driver.is_prepared());
    let events = driver.begin(&EmptyDialogueContext, LocaleResolution::new())?;
    assert!(matches!(&events[0], DialogueEvent::Line(line) if line.text == "Hello."));
    assert_eq!(
        driver
            .begin(&EmptyDialogueContext, LocaleResolution::new())
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::SessionAlreadyActive)
    );
    Ok(())
}

#[test]
fn failed_start_leaves_owner_free_and_second_start_preserves_original()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = asset(":: start default\n> intro@11111111111111111111\n  Hello.\n-> END\n")?;
    let mut driver = SessionDriver::new();
    let result = driver.start_with(
        request(&asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |_| Err::<(), _>("encoder"),
    );
    assert!(matches!(result, Err(DriverError::Output("encoder"))));
    assert!(!driver.has_active_session());
    let initial = driver.start(
        request(&asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let snapshot = driver.snapshot()?;
    assert_eq!(
        driver
            .start(
                request(&asset),
                &EmptyDialogueContext,
                LocaleResolution::new()
            )
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::SessionAlreadyActive)
    );
    assert_eq!(driver.snapshot()?, snapshot);
    assert!(matches!(&initial[0], DialogueEvent::Line(line) if line.text == "Hello."));
    Ok(())
}

#[test]
fn restore_prompt_is_empty_and_blocking_effect_reemits() -> Result<(), Box<dyn std::error::Error>> {
    let prompt_asset = asset(
        ":: start default\n> prompt@11111111111111111111\n  Pick.\n  ? go@22222222222222222222\n    Go.\n    -> END\n",
    )?;
    let mut first = SessionDriver::new();
    let events = first.start(
        request(&prompt_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    assert!(matches!(&events[..], [DialogueEvent::Prompt { .. }]));
    let snapshot = first.snapshot()?;
    let mut restored = SessionDriver::new();
    assert!(
        restored
            .restore(
                &prompt_asset,
                &snapshot,
                &EmptyDialogueContext,
                LocaleResolution::new()
            )?
            .is_empty()
    );

    let effect_asset = asset(
        ":: start default\n! blocking play_sound(chime)\n> after@33333333333333333333\n  After.\n-> END\n",
    )?;
    let mut effect_driver = SessionDriver::new();
    let first_events = effect_driver.start(
        request(&effect_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let effect_snapshot = effect_driver.snapshot()?;
    let mut restored_effect = SessionDriver::new();
    let resumed = restored_effect.restore(
        &effect_asset,
        &effect_snapshot,
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    assert_eq!(first_events, resumed);
    Ok(())
}

#[test]
fn prior_prompt_choice_is_stale_only_after_committed_transition_and_restore()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = asset(
        ":: start default\n? first@11111111111111111111\n  First.\n  -> later\n:: later\n? second@22222222222222222222\n  Second.\n  -> END\n",
    )?;
    let first = ChoiceId::new("11111111111111111111")?;
    let second = ChoiceId::new("22222222222222222222")?;
    let unknown = ChoiceId::new("missing_choice")?;
    let mut driver = SessionDriver::new();
    assert!(matches!(
        &driver.start(
            request(&asset),
            &EmptyDialogueContext,
            LocaleResolution::new()
        )?[..],
        [DialogueEvent::Prompt { .. }]
    ));
    let failed = driver.select_choice_with(
        first.clone(),
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |_| Err::<(), _>("encode failed"),
    );
    assert!(matches!(failed, Err(DriverError::Output("encode failed"))));
    assert_eq!(
        driver
            .select_choice(
                second.clone(),
                &EmptyDialogueContext,
                LocaleResolution::new()
            )
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::InvalidChoice),
        "failed encoder must not publish the next prompt ID"
    );
    assert_eq!(
        driver
            .select_choice(
                unknown.clone(),
                &EmptyDialogueContext,
                LocaleResolution::new()
            )
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::InvalidChoice)
    );
    assert!(matches!(
        &driver.select_choice(
            first.clone(),
            &EmptyDialogueContext,
            LocaleResolution::new()
        )?[..],
        [DialogueEvent::Prompt { .. }]
    ));
    assert_eq!(
        driver
            .select_choice(
                first.clone(),
                &EmptyDialogueContext,
                LocaleResolution::new()
            )
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::StaleChoice)
    );
    let snapshot = driver.snapshot()?;
    let mut restored = SessionDriver::new();
    assert!(
        restored
            .restore(
                &asset,
                &snapshot,
                &EmptyDialogueContext,
                LocaleResolution::new()
            )?
            .is_empty()
    );
    assert_eq!(
        restored
            .select_choice(first, &EmptyDialogueContext, LocaleResolution::new())
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::StaleChoice)
    );
    assert_eq!(
        restored
            .select_choice(unknown, &EmptyDialogueContext, LocaleResolution::new())
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::InvalidChoice)
    );
    driver.end_session()?;
    assert!(matches!(
        &driver.start(
            request(&asset),
            &EmptyDialogueContext,
            LocaleResolution::new()
        )?[..],
        [DialogueEvent::Prompt { .. }]
    ));
    assert_eq!(
        driver
            .select_choice(second, &EmptyDialogueContext, LocaleResolution::new())
            .err()
            .map(|error| error.kind()),
        Some(AdapterErrorKind::InvalidChoice),
        "restart must not retain prior observed IDs"
    );
    assert!(matches!(
        &driver.select_choice(
            ChoiceId::new("11111111111111111111")?,
            &EmptyDialogueContext,
            LocaleResolution::new()
        )?[..],
        [DialogueEvent::Prompt { .. }]
    ));
    Ok(())
}

#[test]
fn failed_choice_and_ack_encoding_keep_session_checkpoint() -> Result<(), Box<dyn std::error::Error>>
{
    let prompt_asset = asset(
        ":: start default\n> prompt@11111111111111111111\n  Pick.\n  ? go@22222222222222222222\n    Go.\n    -> END\n",
    )?;
    let mut driver = SessionDriver::new();
    driver.start(
        request(&prompt_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let before = driver.snapshot()?;
    let choice = recite_core::ChoiceId::new("22222222222222222222")?;
    let rejected = driver.select_choice_with(
        choice.clone(),
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |_| Err::<(), _>("encode"),
    );
    assert!(matches!(rejected, Err(DriverError::Output("encode"))));
    assert_eq!(driver.snapshot()?, before);
    assert!(
        !driver
            .select_choice(choice, &EmptyDialogueContext, LocaleResolution::new())?
            .is_empty()
    );

    let effect_asset = asset(
        ":: start default\n! blocking play_sound(chime)\n> after@33333333333333333333\n  After.\n-> END\n",
    )?;
    let mut effect_driver = SessionDriver::new();
    let events = effect_driver.start(
        request(&effect_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let effect_id = match &events[0] {
        DialogueEvent::Effect(effect) => effect.id.clone(),
        other => panic!("expected effect, got {other:?}"),
    };
    let before = effect_driver.snapshot()?;
    let rejected = effect_driver.acknowledge_effect_with(
        effect_id.clone(),
        recite_runtime::EffectAck::Completed,
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |_| Err::<(), _>("encode"),
    );
    assert!(matches!(rejected, Err(DriverError::Output("encode"))));
    assert_eq!(effect_driver.snapshot()?, before);
    let resumed = effect_driver.acknowledge_effect(
        effect_id,
        recite_runtime::EffectAck::Completed,
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    assert!(matches!(&resumed[0], DialogueEvent::Line(line) if line.text == "After."));
    Ok(())
}
