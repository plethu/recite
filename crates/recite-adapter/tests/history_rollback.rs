mod support;

use recite_adapter::{DriverError, SessionDriver};
use recite_core::ChoiceId;
use recite_runtime::{
    DialogueEvent, EffectAck, EmptyDialogueContext, LocaleResolution,
    snapshot::{decode_session_messagepack, encode_session_messagepack},
};

use support::{asset, request};

#[test]
fn history_appends_roll_back_on_encoding_failure_and_retry_preserves_effect_order()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = asset(concat!(
        ":: start default\n! deferred first()\n",
        "? continue@11111111111111111111\n  Continue.\n  -> later\n",
        ":: later\n! deferred second()\n! immediate notice()\n",
        "! blocking wait()\n! deferred third()\n-> END\n",
    ))?;
    let choice = ChoiceId::new("11111111111111111111")?;
    let mut driver = SessionDriver::new();
    driver.start(
        request(&asset),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    let before = driver.snapshot()?;
    let mut attempted = Vec::new();
    let rejected = driver.select_choice_with(
        choice.clone(),
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |events| {
            attempted = events;
            Err::<(), _>("encode")
        },
    );
    assert!(matches!(rejected, Err(DriverError::Output("encode"))));
    assert_eq!(driver.snapshot()?, before);
    let events = driver.select_choice(choice, &EmptyDialogueContext, LocaleResolution::new())?;
    assert_eq!(events, attempted);
    assert!(
        matches!(&events[..], [DialogueEvent::Effect(first), DialogueEvent::Effect(second)]
        if first.function == "notice" && second.function == "wait")
    );
    let DialogueEvent::Effect(blocking) = &events[1] else {
        panic!("blocking effect");
    };
    let blocked = driver.snapshot()?;
    let session = decode_session_messagepack(asset.dialogue(), &blocked)?;
    assert_eq!(
        session.selected_choice_history(),
        [ChoiceId::new("11111111111111111111")?]
    );
    assert_eq!(
        session
            .deferred_effects()
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let rejected = driver.acknowledge_effect_with(
        blocking.id.clone(),
        EffectAck::Completed,
        &EmptyDialogueContext,
        LocaleResolution::new(),
        |_| Err::<(), _>("encode end"),
    );
    assert!(matches!(rejected, Err(DriverError::Output("encode end"))));
    assert_eq!(driver.snapshot()?, blocked);
    let mut restored = SessionDriver::new();
    let replay = restored.restore(
        &asset,
        &blocked,
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    assert_eq!(replay, [DialogueEvent::Effect(blocking.clone())]);
    let end = driver.acknowledge_effect(
        blocking.id.clone(),
        EffectAck::Completed,
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?;
    assert_eq!(
        end,
        restored.acknowledge_effect(
            blocking.id.clone(),
            EffectAck::Completed,
            &EmptyDialogueContext,
            LocaleResolution::new()
        )?
    );
    let [DialogueEvent::End { deferred_effects }] = &end[..] else {
        panic!("ending event");
    };
    assert_eq!(
        deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["first", "second", "third"]
    );
    assert_eq!(driver.snapshot()?, restored.snapshot()?);
    let original = decode_session_messagepack(asset.dialogue(), &before)?;
    assert_eq!(encode_session_messagepack(&original)?, before);
    Ok(())
}
