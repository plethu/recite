#[path = "support/preview.rs"]
mod preview_support;

use std::cell::Cell;

use preview_support::asset;
use recite_runtime::{
    ConditionAnswer, ConditionValue, LocaleError, LocaleProvider, PluralResolution,
    PreviewConditionResult, PreviewError, PreviewEvent, PreviewInputs, PreviewOptions,
    PreviewSession, PreviewStatus, TextDomain,
};

struct CountingProvider {
    calls: Cell<usize>,
}

impl LocaleProvider for CountingProvider {
    fn lookup(
        &self,
        _id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<Option<String>, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        Ok(None)
    }

    fn resolve_plural(
        &self,
        _id: &str,
        _source_singular: &str,
        _source_plural: &str,
        _count: i64,
        _domain: TextDomain,
        _locale: &recite_core::LocaleId,
        _variant: Option<&str>,
    ) -> Result<PluralResolution, LocaleError> {
        self.calls.set(self.calls.get() + 1);
        Ok(PluralResolution {
            template: None,
            selected_arm: None,
            matched_locale: None,
            matched_context: None,
            matched_key: None,
            attempts: Vec::new(),
        })
    }
}

#[test]
fn preview_projects_lines_end_and_transcript_without_duplicate_condition_control() {
    let asset = asset(":: start default\n> hello@12345678901234567890\n  Hello.\n-> END\n");
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");

    let line = preview.step(PreviewInputs::new());
    assert!(matches!(line.events(), [PreviewEvent::Line(_)]));
    let end = preview.step(PreviewInputs::new());
    assert!(matches!(end.events(), [PreviewEvent::End { .. }]));
    assert_eq!(preview.trace().events().len(), 2);
    assert_eq!(preview.transcript().events().len(), 2);
    assert!(matches!(preview.state().status(), PreviewStatus::Ended));
}

#[test]
fn source_only_preview_bypasses_locale_provider() {
    let asset = asset(":: start default\n> hello@12345678901234567890\n  Hello.\n-> END\n");
    let provider = CountingProvider {
        calls: Cell::new(0),
    };
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let output = preview.step(PreviewInputs::new().with_locale_provider(&provider));
    assert!(matches!(output.events(), [PreviewEvent::Line(_)]));
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(preview.state().locale(), None);
}

#[test]
fn restart_is_explicit_and_changed_payload_is_not_swapped_silently() {
    let asset = asset(":: start default\n> hello@12345678901234567890\n  Hello.\n-> END\n");
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    preview.step(PreviewInputs::new());
    let restarted = preview.dispatch(
        recite_runtime::PreviewCommand::Restart,
        PreviewInputs::new(),
    );
    assert!(matches!(
        restarted.events(),
        [PreviewEvent::Restarted { .. }]
    ));
    assert!(matches!(preview.state().status(), PreviewStatus::Ready));

    let mut candidate = asset.clone();
    candidate.lines[0].source_text = "A different payload.".to_owned();
    candidate.lines[0].authored_source_text = "A different payload.".to_owned();
    let before = preview.session().clone();
    let changed = preview.assess_asset(&candidate).expect("assess");
    assert!(matches!(
        changed.events(),
        [PreviewEvent::RestartRequired { .. }]
    ));
    assert_eq!(*preview.session(), before);
}

#[test]
fn condition_answer_replays_transactionally_and_rejects_wrong_id_or_type() {
    let asset = asset(concat!(
        ":: start default\n",
        ":if trusts(player)\n",
        "  > yes@12345678901234567890\n    Yes.\n",
        ":else\n",
        "  > no@12345678901234567891\n    No.\n",
        "-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let request = match preview.step(PreviewInputs::new()).events() {
        [PreviewEvent::ConditionRequested(request)] => request.clone(),
        events => panic!("expected one condition request, got {events:?}"),
    };
    let before = preview.session().clone();
    let wrong = preview.answer(
        (request.id().get() + 1).into(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert!(matches!(
        wrong.events(),
        [PreviewEvent::Error(
            PreviewError::ConditionRequestMismatch { .. }
        )]
    ));
    assert_eq!(*preview.session(), before);
    let wrong_type = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::EnumVariant("ready".to_owned())),
        PreviewInputs::new(),
    );
    assert!(matches!(
        wrong_type.events(),
        [PreviewEvent::Error(
            PreviewError::ConditionResultTypeMismatch { .. }
        )]
    ));
    assert_eq!(*preview.session(), before);
    let line = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert!(matches!(
        line.events(),
        [
            PreviewEvent::ConditionResult {
                result: PreviewConditionResult::Value(ConditionValue::Bool(true)),
                ..
            },
            PreviewEvent::Line(_)
        ]
    ));
}

#[test]
fn changed_input_revision_refuses_replay_without_mutating_session() {
    let asset = asset(
        ":: start default\n:if trusts(player)\n  > yes@12345678901234567890\n    Yes.\n-> END\n",
    );
    let first_provider = CountingProvider {
        calls: Cell::new(0),
    };
    let changed_provider = CountingProvider {
        calls: Cell::new(0),
    };
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let first = preview.step(
        PreviewInputs::new()
            .with_locale_provider(&first_provider)
            .with_revision(1),
    );
    let request = match &first.events()[0] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected request, got {event:?}"),
    };
    let before = preview.session().clone();
    let output = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new()
            .with_locale_provider(&changed_provider)
            .with_revision(2),
    );
    assert!(matches!(
        output.events(),
        [PreviewEvent::Error(
            PreviewError::InputRevisionMismatch { .. }
        )]
    ));
    assert_eq!(*preview.session(), before);
}

#[test]
fn sequential_conditions_and_short_circuit_replay_in_source_order() {
    let asset = asset(concat!(
        ":: start default\n",
        ":if trusts(player) and has_key(player)\n",
        "  > yes@12345678901234567890\n    Yes.\n",
        ":else\n",
        "  > no@12345678901234567891\n    No.\n",
        "-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let first = preview.step(PreviewInputs::new().with_revision(7));
    let first_request = match &first.events()[0] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected first request, got {event:?}"),
    };
    assert_eq!(first_request.query().function(), "trusts");
    let second = preview.answer(
        first_request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new().with_revision(7),
    );
    let second_request = match &second.events()[1] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected second request, got {event:?}"),
    };
    assert_eq!(second_request.query().function(), "has_key");
    let line = preview.answer(
        second_request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(false)),
        PreviewInputs::new().with_revision(7),
    );
    assert!(matches!(line.events().last(), Some(PreviewEvent::Line(_))));
    assert_eq!(preview.trace().events().len(), 5);
}

#[test]
fn availability_answer_is_in_prompt_projection_and_locked_choice_cannot_advance() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890\n  Choose.\n",
        "  ? locked@12345678901234567891 requires=(trusts(player))\n",
        "    Locked.\n    -> END\n",
        "  ? leave@12345678901234567892\n",
        "    Leave.\n    -> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let requested = preview.step(PreviewInputs::new().with_revision(3));
    let request = match &requested.events()[0] {
        PreviewEvent::ConditionRequested(request) => request.clone(),
        event => panic!("expected availability request, got {event:?}"),
    };
    assert_eq!(
        request.prompt().map(|prompt| prompt.choices().len()),
        Some(2)
    );
    let prompt = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(false)),
        PreviewInputs::new().with_revision(3),
    );
    let locked = match prompt.events().last() {
        Some(PreviewEvent::Prompt(prompt)) => prompt.identity().choices()[0].clone(),
        event => panic!("expected prompt, got {event:?}"),
    };
    let before = preview.session().clone();
    let rejected = preview.choose(locked, PreviewInputs::new().with_revision(3));
    assert!(matches!(
        rejected.events(),
        [PreviewEvent::Error(PreviewError::Runtime(_))]
    ));
    assert_eq!(*preview.session(), before);
}

#[test]
fn divert_prompt_provenance_uses_trial_block() {
    let asset = asset(concat!(
        ":: start default\n",
        "-> target\n",
        ":: target\n",
        "> target_line@12345678901234567890\n  Target.\n",
        "  ? target_choice@12345678901234567891\n    Continue.\n    -> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let output = preview.step(PreviewInputs::new());
    let PreviewEvent::Prompt(prompt) = &output.events()[0] else {
        panic!("expected target prompt: {:?}", output.events());
    };
    assert_eq!(prompt.identity().block().as_str(), "target");
}

#[test]
fn divert_condition_provenance_uses_trial_block() {
    let asset = asset(concat!(
        ":: start default\n",
        "-> gated\n",
        ":: gated\n",
        ":if trusts(player)\n",
        "  > yes@12345678901234567890\n    Yes.\n",
        "-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let output = preview.step(PreviewInputs::new());
    let PreviewEvent::ConditionRequested(request) = &output.events()[0] else {
        panic!("expected target condition: {:?}", output.events());
    };
    assert_eq!(request.block().as_str(), "gated");
}

#[test]
fn successful_enum_condition_replay_is_typed_and_deterministic() {
    let asset = asset(concat!(
        ":: start default\n",
        ":match mood()\n",
        "  :case ready\n",
        "    > ready@12345678901234567890\n      Ready.\n",
        "  :case _\n",
        "    > other@12345678901234567891\n      Other.\n",
        "-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let request = match preview.step(PreviewInputs::new()).events() {
        [PreviewEvent::ConditionRequested(request)] => request.clone(),
        events => panic!("expected enum query, got {events:?}"),
    };
    assert_eq!(
        request.query().expected_type(),
        recite_runtime::ConditionExpectedType::Enum
    );
    let output = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::EnumVariant("ready".to_owned())),
        PreviewInputs::new(),
    );
    assert!(matches!(
        output.events(),
        [PreviewEvent::ConditionResult {
            result: PreviewConditionResult::Value(ConditionValue::EnumVariant(value)),
            ..
        }, PreviewEvent::Line(line)] if value == "ready" && line.text == "Ready."
    ));
}
