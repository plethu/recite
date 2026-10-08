mod support;

use std::fmt::Write;

use recite_adapter::{AdapterErrorKind, SessionDriver};
use recite_core::ChoiceId;
use recite_runtime::{DialogueEvent, EffectAck, EmptyDialogueContext, LocaleResolution};

use support::{asset, request};

const LINE_LOOP: &str = ":: looping\n> repeated@11111111111111111111\n  Again.\n-> looping\n";

#[test]
fn line_and_immediate_effect_loops_fail_without_committing()
-> Result<(), Box<dyn std::error::Error>> {
    for source in [
        format!(":: start default\n-> looping\n{LINE_LOOP}"),
        ":: start default\n! immediate play_sound(chime)\n-> start\n".to_owned(),
    ] {
        let asset = asset(&source)?;
        let mut driver = SessionDriver::new();
        let error = driver
            .start(
                request(&asset),
                &EmptyDialogueContext,
                LocaleResolution::new(),
            )
            .expect_err("an endless batch must fail");
        assert_eq!(error.kind(), AdapterErrorKind::DialogueFault);
        assert!(!driver.has_active_session(), "failed start frees its owner");

        driver.prepare(request(&asset))?;
        let checkpoint = driver.snapshot()?;
        let mut restored = SessionDriver::new();
        restored.prepare_restore(&asset, &checkpoint)?;
        for owner in [&mut driver, &mut restored] {
            let error = owner
                .begin(&EmptyDialogueContext, LocaleResolution::new())
                .expect_err("begin must also bound a restored session");
            assert_eq!(error.kind(), AdapterErrorKind::DialogueFault);
            assert!(owner.is_prepared());
            assert_eq!(owner.snapshot()?, checkpoint, "failed begin rolls back");
        }
    }
    Ok(())
}

#[test]
fn choice_and_acknowledgement_loops_keep_the_pending_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let choice_asset = asset(&format!(
        ":: start default\n> prompt@22222222222222222222\n  Pick.\n  ? again@33333333333333333333\n    Again.\n    -> looping\n{LINE_LOOP}"
    ))?;
    let mut driver = SessionDriver::new();
    driver.start(
        request(&choice_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let checkpoint = driver.snapshot()?;
    let error = driver
        .select_choice(
            ChoiceId::new("33333333333333333333")?,
            &EmptyDialogueContext,
            LocaleResolution::new(),
        )
        .expect_err("choice cannot drain a line loop");
    assert_eq!(error.kind(), AdapterErrorKind::DialogueFault);
    assert_eq!(driver.snapshot()?, checkpoint);

    let effect_asset = asset(&format!(
        ":: start default\n! blocking play_sound(chime)\n-> looping\n{LINE_LOOP}"
    ))?;
    let mut driver = SessionDriver::new();
    let events = driver.start(
        request(&effect_asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let DialogueEvent::Effect(effect) = &events[0] else {
        panic!("expected the blocking effect");
    };
    let checkpoint = driver.snapshot()?;
    let error = driver
        .acknowledge_effect(
            effect.id.clone(),
            EffectAck::Completed,
            &EmptyDialogueContext,
            LocaleResolution::new(),
        )
        .expect_err("acknowledgement cannot drain a line loop");
    assert_eq!(error.kind(), AdapterErrorKind::DialogueFault);
    assert_eq!(driver.snapshot()?, checkpoint);
    Ok(())
}

#[test]
fn terminal_event_at_the_batch_limit_succeeds_and_one_more_fails()
-> Result<(), Box<dyn std::error::Error>> {
    // 9,999 lines followed by one terminal event is exactly 10,000 events.
    let mut source = String::from(":: start default\n");
    for id in 0..9_999 {
        writeln!(&mut source, "> line{id}@{id:020}\n  Line.")?;
    }
    for terminal in [
        "-> END\n",
        "! blocking play_sound(chime)\n-> END\n",
        "> prompt@99999999999999999999\n  Pick.\n  ? go@88888888888888888888\n    Go.\n    -> END\n",
    ] {
        let asset = asset(&format!("{source}{terminal}"))?;
        let mut driver = SessionDriver::new();
        let events = driver.start(
            request(&asset),
            &EmptyDialogueContext,
            LocaleResolution::new(),
        )?;
        assert_eq!(events.len(), 10_000);
        assert!(!matches!(events.last(), Some(DialogueEvent::Line(_))));
    }
    source.push_str("> extra@77777777777777777777\n  Too many.\n-> END\n");
    let asset = asset(&source)?;
    let mut driver = SessionDriver::new();
    let error = driver
        .start(
            request(&asset),
            &EmptyDialogueContext,
            LocaleResolution::new(),
        )
        .expect_err("terminal event would exceed the batch bound");
    assert_eq!(error.kind(), AdapterErrorKind::DialogueFault);
    assert!(!driver.has_active_session());
    Ok(())
}
