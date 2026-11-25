//! Fixture, save points and traversal helpers shared by the cross-revision tests.

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, EffectId,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueError, DialogueEvent,
    DialogueSession, EffectAck, acknowledge_effect, choose, next,
    resume::{ResumeFallback, ResumeOptions, ResumeReport, SessionResume, resume_session},
    snapshot::{DialogueSessionSnapshot, snapshot_session},
    start_scene,
};

pub(super) const BASE: &str = concat!(
    ":: start default\n",
    "> @l_intro\n",
    "  Intro.\n",
    "! immediate chime()\n",
    "> @l_second\n",
    "  Second.\n",
    ":if trusts(player)\n",
    "  > @l_inside\n",
    "    Inside.\n",
    "  ! blocking grant(map)\n",
    "  > @l_after_grant\n",
    "    After grant.\n",
    ":else\n",
    "  > @l_else\n",
    "    Else.\n",
    ":match mood()\n",
    "  :case tired\n",
    "    > @l_tired\n",
    "      Tired.\n",
    "  :case _\n",
    "    > @l_other\n",
    "      Other.\n",
    "! deferred remember(start)\n",
    "> @l_prompt\n",
    "  Where now?\n",
    "  ? @c_work\n",
    "    Work.\n",
    "    -> work\n",
    "  ? @c_leave\n",
    "    Leave.\n",
    "    -> END\n",
    "\n",
    ":: work\n",
    "> @l_work\n",
    "  Working.\n",
    "> @l_work_done\n",
    "  Done working.\n",
    "-> END\n",
);

pub(super) const OTHER: &str = ":: elsewhere\n> @l_elsewhere\n  Elsewhere.\n-> END\n";

/// Where a save was taken.
#[derive(Clone, Copy, Debug)]
pub(super) enum SavePoint {
    Start,
    AfterIntro,
    AfterChime,
    InsideIf,
    PendingGrant,
    InsideMatch,
    PendingPrompt,
    InWorkBlock,
    Ended,
}

pub(super) const ALL_POINTS: [SavePoint; 9] = [
    SavePoint::Start,
    SavePoint::AfterIntro,
    SavePoint::AfterChime,
    SavePoint::InsideIf,
    SavePoint::PendingGrant,
    SavePoint::InsideMatch,
    SavePoint::PendingPrompt,
    SavePoint::InWorkBlock,
    SavePoint::Ended,
];

pub(super) fn context(
    query: ConditionQuery<'_>,
) -> Result<ConditionValue, ConditionEvaluationError> {
    Ok(match query.function() {
        "mood" => ConditionValue::EnumVariant("tired".to_owned()),
        _ => ConditionValue::Bool(true),
    })
}

pub(super) fn compile_with(base: &str, other: &str, compiler: &str) -> CompiledDialogue {
    let report = compile_inputs(
        [
            CompileInput::new("dialogue/a.recite", base),
            CompileInput::new("dialogue/b.recite", other),
        ],
        CompileOptions::new(
            CompilerVersion::new(compiler).expect("compiler version"),
            CompiledAssetId::new("dialogue/main.recitec").expect("asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("source map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compile does not hard fail");
    assert!(
        report.diagnostics.is_empty(),
        "fixture compiles cleanly: {:?}",
        report.diagnostics
    );
    report.asset.expect("asset").dialogue
}

pub(super) fn compile(base: &str) -> CompiledDialogue {
    compile_with(base, OTHER, "0.0.1")
}

pub(super) fn edited(from: &str, to: &str) -> CompiledDialogue {
    assert!(BASE.contains(from), "edit anchor `{from}` exists");
    compile(&BASE.replacen(from, to, 1))
}

pub(super) fn step(asset: &CompiledDialogue, session: &mut DialogueSession) -> DialogueEvent {
    next(asset, session, &context).expect("advances")
}

/// Drives a fresh session on `asset` to `point`.
pub(super) fn drive(asset: &CompiledDialogue, point: SavePoint) -> DialogueSession {
    let mut session = start_scene(asset, None).expect("starts");
    let until_line = |session: &mut DialogueSession, id: &str| loop {
        match step(asset, session) {
            DialogueEvent::Line(line) if line.id.as_str() == id => break,
            DialogueEvent::Effect(effect)
                if effect.mode == recite_runtime::DialogueEffectMode::Blocking =>
            {
                acknowledge_effect(session, effect.id, EffectAck::Completed).expect("acks");
            }
            DialogueEvent::End { .. } | DialogueEvent::Prompt { .. } => {
                panic!("did not reach line `{id}`")
            }
            _ => {}
        }
    };
    match point {
        SavePoint::Start => {}
        SavePoint::AfterIntro => until_line(&mut session, "l_intro"),
        SavePoint::AfterChime => {
            until_line(&mut session, "l_intro");
            assert!(matches!(
                step(asset, &mut session),
                DialogueEvent::Effect(_)
            ));
        }
        SavePoint::InsideIf => until_line(&mut session, "l_inside"),
        SavePoint::PendingGrant => {
            until_line(&mut session, "l_inside");
            let DialogueEvent::Effect(effect) = step(asset, &mut session) else {
                panic!("expected blocking effect");
            };
            assert_eq!(effect.function, "grant");
        }
        SavePoint::InsideMatch => until_line(&mut session, "l_tired"),
        SavePoint::PendingPrompt => {
            until_line(&mut session, "l_tired");
            while !matches!(step(asset, &mut session), DialogueEvent::Prompt { .. }) {}
        }
        SavePoint::InWorkBlock => {
            let mut session_at_prompt = drive(asset, SavePoint::PendingPrompt);
            let event = choose(
                asset,
                &mut session_at_prompt,
                ChoiceId::new("c_work").expect("choice"),
                &context,
            )
            .expect("chooses");
            assert!(matches!(event, DialogueEvent::Line(ref line) if line.id.as_str() == "l_work"));
            return session_at_prompt;
        }
        SavePoint::Ended => {
            let mut session_at_prompt = drive(asset, SavePoint::PendingPrompt);
            let event = choose(
                asset,
                &mut session_at_prompt,
                ChoiceId::new("c_leave").expect("choice"),
                &context,
            )
            .expect("chooses");
            assert!(matches!(event, DialogueEvent::End { .. }));
            return session_at_prompt;
        }
    }
    session
}

pub(super) fn save(asset: &CompiledDialogue, point: SavePoint) -> DialogueSessionSnapshot {
    snapshot_session(asset, &drive(asset, point)).expect("snapshots")
}

pub(super) fn resume(asset: &CompiledDialogue, snapshot: DialogueSessionSnapshot) -> SessionResume {
    resume_session(asset, snapshot, &ResumeOptions::new()).expect("resume is not an error")
}

pub(super) fn resumed(outcome: SessionResume) -> (DialogueSession, ResumeReport) {
    match outcome {
        SessionResume::Resumed { session, report } => (session, report),
        SessionResume::Fallback(fallback) => panic!("expected resume, got {fallback:?}"),
        _ => unreachable!("unknown outcome"),
    }
}

pub(super) fn fallback(outcome: SessionResume) -> ResumeFallback {
    match outcome {
        SessionResume::Fallback(fallback) => fallback,
        SessionResume::Resumed { report, .. } => panic!("expected fallback, got {report:?}"),
        _ => unreachable!("unknown outcome"),
    }
}

/// Plays a session to its end, choosing the first available choice and
/// acknowledging every blocking effect. Effect request IDs carry a
/// traversal counter that differs between saved and fresh sessions, so it is
/// removed before comparison.
pub(super) fn play_out(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
) -> Vec<DialogueEvent> {
    let mut events = Vec::new();
    for _ in 0..64 {
        let event = match next(asset, session, &context) {
            Ok(event) => event,
            Err(DialogueError::EffectPending { effect }) => {
                acknowledge_effect(session, effect, EffectAck::Completed).expect("acks");
                continue;
            }
            Err(DialogueError::SessionEnded) => return events,
            // A fresh session driven to a prompt has already presented it.
            Err(DialogueError::PromptPending { choices }) => {
                let after = choose(asset, session, choices[0].clone(), &context).expect("chooses");
                let ended = matches!(after, DialogueEvent::End { .. });
                events.push(normalize(after));
                if ended {
                    return events;
                }
                continue;
            }
            Err(error) => panic!("traversal failed: {error}"),
        };
        match &event {
            DialogueEvent::Prompt { choices, .. } => {
                let choice = choices
                    .iter()
                    .find(|choice| choice.availability.is_available)
                    .expect("an available choice")
                    .id
                    .clone();
                events.push(normalize(event.clone()));
                let after = choose(asset, session, choice, &context).expect("chooses");
                if let DialogueEvent::Effect(effect) = &after
                    && effect.mode == recite_runtime::DialogueEffectMode::Blocking
                {
                    acknowledge_effect(session, effect.id.clone(), EffectAck::Completed)
                        .expect("acks");
                }
                let ended = matches!(after, DialogueEvent::End { .. });
                events.push(normalize(after));
                if ended {
                    return events;
                }
            }
            DialogueEvent::Effect(effect)
                if effect.mode == recite_runtime::DialogueEffectMode::Blocking =>
            {
                acknowledge_effect(session, effect.id.clone(), EffectAck::Completed).expect("acks");
                events.push(normalize(event));
            }
            DialogueEvent::End { .. } => {
                events.push(normalize(event));
                return events;
            }
            _ => events.push(normalize(event)),
        }
    }
    panic!("traversal did not end");
}

pub(super) fn normalize(event: DialogueEvent) -> DialogueEvent {
    let strip = |id: &EffectId| {
        EffectId::new(id.as_str().split('#').next().expect("prefix")).expect("effect id")
    };
    match event {
        DialogueEvent::Effect(mut effect) => {
            effect.id = strip(&effect.id);
            DialogueEvent::Effect(effect)
        }
        DialogueEvent::End {
            mut deferred_effects,
        } => {
            // Effects collected before a save keep their saved request,
            // including its positional ID and span; compare what they ask.
            for effect in &mut deferred_effects {
                effect.id = EffectId::new("deferred").expect("effect id");
                effect.source_span = recite_core::SourceSpan::new(
                    "deferred",
                    recite_core::SourcePosition::new(1, 1).expect("position"),
                    None,
                );
            }
            DialogueEvent::End { deferred_effects }
        }
        other => other,
    }
}

/// A resumed session continues like a fresh traversal of the patched asset
/// driven to the same point, apart from deferred effects collected before the
/// save, which keep their saved request.
pub(super) fn assert_continues_like_fresh(
    patched: &CompiledDialogue,
    point: SavePoint,
    mut session: DialogueSession,
) {
    let mut fresh = drive(patched, point);
    if matches!(point, SavePoint::PendingPrompt) {
        // A restored prompt, kept or evaluated again, is presented on the
        // next advance; the fresh session presented it while being driven.
        let presented = next(patched, &mut session, &context).expect("presents prompt");
        assert!(matches!(presented, DialogueEvent::Prompt { .. }));
    }
    if let Some(effect) = session.pending_effect().cloned() {
        let reemitted = next(patched, &mut session, &context).expect("re-emits");
        assert_eq!(reemitted, DialogueEvent::Effect(effect.clone()));
        acknowledge_effect(&mut session, effect.id, EffectAck::Completed).expect("acks saved id");
        let fresh_effect = fresh.pending_effect().expect("fresh pending").id.clone();
        acknowledge_effect(&mut fresh, fresh_effect, EffectAck::Completed).expect("acks fresh");
    }
    assert_eq!(
        play_out(patched, &mut session),
        play_out(patched, &mut fresh)
    );
}
