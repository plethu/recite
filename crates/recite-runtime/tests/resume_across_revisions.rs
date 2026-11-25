#![cfg(test)]
//! Saves taken against one compiled revision, resumed against patched ones.
//!
//! Every save point kind is resumed against every edit kind. Resumed sessions
//! must continue exactly like a fresh traversal of the patched asset from the
//! same point; fallbacks must return the saved effects untouched.

use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, LineId,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, ContentFingerprint, SchemaFingerprint,
        SourceMapId,
    },
};
use recite_runtime::{
    DialogueError, DialogueEvent, EffectAck, acknowledge_effect, choose,
    resume::{
        PromptResume, ResumeFallbackReason, ResumeLandmark, ResumeLevel, ResumeOptions,
        resume_session, resume_session_messagepack,
    },
    snapshot::encode_session_messagepack,
};

#[path = "resume_across_revisions/support.rs"]
mod support;
use support::*;

#[test]
fn identical_rebuild_restores_every_point_exactly() {
    let base = compile(BASE);
    let rebuilt = compile(BASE);
    for point in ALL_POINTS {
        let (session, report) = resumed(resume(&rebuilt, save(&base, point)));
        assert_eq!(report.level, ResumeLevel::Exact, "{point:?}");
        assert!(!report.schema_changed && !report.compiler_changed);
        if !matches!(point, SavePoint::Ended) {
            assert_continues_like_fresh(&rebuilt, point, session);
        }
    }
}

#[test]
fn compiler_update_and_unrelated_file_edit_keep_every_point() {
    let base = compile(BASE);
    let compiler = compile_with(BASE, OTHER, "0.0.2");
    let other_file = compile_with(
        BASE,
        &OTHER.replace("Elsewhere.", "Somewhere else."),
        "0.0.1",
    );
    for (patched, compiler_changed) in [(&compiler, true), (&other_file, false)] {
        for point in ALL_POINTS {
            let (session, report) = resumed(resume(patched, save(&base, point)));
            assert_eq!(report.level, ResumeLevel::UnchangedBlock, "{point:?}");
            assert_eq!(report.compiler_changed, compiler_changed);
            if !matches!(point, SavePoint::Ended) {
                assert_continues_like_fresh(patched, point, session);
            }
        }
    }
}

#[test]
fn schema_change_alone_keeps_every_point_and_is_reported() {
    let base = compile(BASE);
    let mut payload = base.clone().into_payload();
    payload.header.schema_fingerprint = SchemaFingerprint::Fingerprint(
        ContentFingerprint::blake3(vec![7; 32]).expect("fingerprint"),
    );
    let patched = CompiledDialogue::new(payload);
    for point in ALL_POINTS {
        let (_, report) = resumed(resume(&patched, save(&base, point)));
        assert_eq!(report.level, ResumeLevel::UnchangedBlock, "{point:?}");
        assert!(report.schema_changed);
    }
}

#[test]
fn text_edit_resumes_in_place_and_shows_the_new_text() {
    let base = compile(BASE);
    let patched = edited("  Second.\n", "  Second, corrected.\n");
    for point in ALL_POINTS {
        let (session, report) = resumed(resume(&patched, save(&base, point)));
        let expected = if matches!(point, SavePoint::InWorkBlock) {
            ResumeLevel::UnchangedBlock
        } else {
            ResumeLevel::TextEdited
        };
        assert_eq!(report.level, expected, "{point:?}");
        if !matches!(point, SavePoint::Ended) {
            assert_continues_like_fresh(&patched, point, session);
        }
    }
    let (mut session, _) = resumed(resume(&patched, save(&base, SavePoint::AfterChime)));
    let DialogueEvent::Line(line) = step(&patched, &mut session) else {
        panic!("expected the edited line");
    };
    assert_eq!(line.text, "Second, corrected.");
}

#[test]
fn inserted_lines_relocate_saves_before_and_after_them() {
    let base = compile(BASE);
    let before_save = edited("> @l_intro\n", "> @l_new_first\n  New first.\n> @l_intro\n");
    let after_save = edited(
        "! deferred remember(start)\n",
        "> @l_new_late\n  New late line.\n! deferred remember(start)\n",
    );
    for patched in [&before_save, &after_save] {
        for point in ALL_POINTS {
            let outcome = resume(patched, save(&base, point));
            if matches!(point, SavePoint::Start) {
                let (_, report) = resumed(outcome);
                assert_eq!(report.level, ResumeLevel::Relocated);
                continue;
            }
            let (session, report) = resumed(outcome);
            let expected = if matches!(point, SavePoint::InWorkBlock) {
                ResumeLevel::UnchangedBlock
            } else {
                ResumeLevel::Relocated
            };
            assert_eq!(report.level, expected, "{point:?}");
            if !matches!(point, SavePoint::Ended) {
                assert_continues_like_fresh(patched, point, session);
            }
        }
    }
    // A save after the inserted line's position plays it next.
    let (mut session, _) = resumed(resume(&after_save, save(&base, SavePoint::InsideMatch)));
    let DialogueEvent::Line(line) = step(&after_save, &mut session) else {
        panic!("expected the inserted line");
    };
    assert_eq!(line.id.as_str(), "l_new_late");
}

#[test]
fn pending_prompt_keeps_saved_choices_or_is_evaluated_again() {
    let base = compile(BASE);
    let saved = save(&base, SavePoint::PendingPrompt);

    let reworded = edited("    Work.\n", "    Get to work.\n");
    let (mut session, report) = resumed(resume(&reworded, saved.clone()));
    assert_eq!(report.level, ResumeLevel::TextEdited);
    assert_eq!(report.prompt, PromptResume::Kept);
    let DialogueEvent::Prompt { choices, .. } = step(&reworded, &mut session) else {
        panic!("restored prompt is presented again");
    };
    assert_eq!(choices[0].text, "Get to work.");

    let added = edited(
        "  ? @c_leave\n",
        "  ? @c_rest\n    Rest.\n    -> END\n  ? @c_leave\n",
    );
    let removed = edited("  ? @c_work\n    Work.\n    -> work\n", "");
    for patched in [&added, &removed] {
        let (session, report) = resumed(resume(patched, saved.clone()));
        assert_eq!(report.level, ResumeLevel::Relocated);
        assert_eq!(report.prompt, PromptResume::Reevaluated);
        assert_continues_like_fresh(patched, SavePoint::PendingPrompt, session);
    }
    let (mut session, _) = resumed(resume(&added, saved));
    let DialogueEvent::Prompt { choices, .. } = step(&added, &mut session) else {
        panic!("re-evaluated prompt is presented");
    };
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["c_work", "c_rest", "c_leave"]
    );
}

#[test]
fn changed_effect_before_the_save_point_falls_back_with_saved_effects() {
    let base = compile(BASE);
    let patched = edited("! immediate chime()\n", "! immediate chime(loud)\n");
    let fell_back = fallback(resume(&patched, save(&base, SavePoint::AfterChime)));
    assert_eq!(
        fell_back.reason,
        ResumeFallbackReason::StatementsChanged {
            landmark: Some(ResumeLandmark::Line(LineId::new("l_intro").expect("id"))),
        }
    );
    assert!(fell_back.block_available);
    assert_eq!(fell_back.block_id.as_str(), "start");

    // The blocking effect itself changed: the saved request comes back with
    // its original ID so the game can reconcile it.
    let grant_changed = edited("! blocking grant(map)\n", "! blocking grant(key)\n");
    let saved = save(&base, SavePoint::PendingGrant);
    let saved_request = saved.pending_effect.clone().expect("pending").request;
    let fell_back = fallback(resume(&grant_changed, saved));
    let pending = fell_back.pending_effect.expect("pending effect returned");
    assert_eq!(pending.id.as_str(), saved_request.id);
    assert_eq!(pending.function, "grant");
    assert!(matches!(
        fell_back.reason,
        ResumeFallbackReason::StatementsChanged { .. }
    ));
}

#[test]
fn fallback_returns_deferred_effects_and_can_restart_the_block() {
    let base = compile(BASE);
    let patched = compile(
        &BASE
            .replacen("> @l_prompt\n", "> @l_prompt_new\n", 1)
            .replacen("? @c_work\n", "? @c_work_new\n", 1)
            .replacen("? @c_leave\n", "? @c_leave_new\n", 1),
    );
    let fell_back = fallback(resume(&patched, save(&base, SavePoint::PendingPrompt)));
    assert_eq!(
        fell_back
            .deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["remember"]
    );
    assert!(matches!(
        fell_back.reason,
        ResumeFallbackReason::LandmarkRemoved { .. }
    ));
    let mut restarted = fell_back.restart_block(&patched).expect("restarts");
    let DialogueEvent::Line(line) = step(&patched, &mut restarted) else {
        panic!("restart begins the saved block");
    };
    assert_eq!(line.id.as_str(), "l_intro");
    assert!(restarted.deferred_effects().is_empty());
}

#[test]
fn deferred_effects_collected_before_the_save_are_delivered_as_saved() {
    let base = compile(BASE);
    // Moving the effect changes its positional compiled ID.
    let patched = edited("> @l_intro\n", "\n\n> @l_intro\n");
    let saved = save(&base, SavePoint::PendingPrompt);
    let saved_effect = saved.deferred_effects[0].clone();
    let (mut session, _) = resumed(resume(&patched, saved));
    step(&patched, &mut session);
    let end = choose(
        &patched,
        &mut session,
        ChoiceId::new("c_leave").expect("choice"),
        &context,
    )
    .expect("chooses");
    let DialogueEvent::End { deferred_effects } = end else {
        panic!("expected end");
    };
    assert_eq!(deferred_effects.len(), 1);
    assert_eq!(deferred_effects[0].id.as_str(), saved_effect.id);
    assert_eq!(deferred_effects[0].source_span.start.line(), 23);
}

#[test]
fn changed_branch_condition_or_arm_falls_back() {
    let base = compile(BASE);
    let condition = edited(":if trusts(player)\n", ":if trusts(guard)\n");
    for point in [SavePoint::InsideIf, SavePoint::PendingGrant] {
        let fell_back = fallback(resume(&condition, save(&base, point)));
        assert!(matches!(
            fell_back.reason,
            ResumeFallbackReason::StatementsChanged { .. }
        ));
    }
    let arm = edited("  :case tired\n", "  :case sleepy\n");
    let fell_back = fallback(resume(&arm, save(&base, SavePoint::InsideMatch)));
    assert_eq!(fell_back.reason, ResumeFallbackReason::BranchRemoved);
    // Saves outside the edited branch are unaffected by where it sits.
    let (_, report) = resumed(resume(&arm, save(&base, SavePoint::PendingPrompt)));
    assert_eq!(report.level, ResumeLevel::Relocated);
}

#[test]
fn removed_or_moved_landmark_and_removed_block_fall_back() {
    let base = compile(BASE);
    let removed = edited("> @l_intro\n  Intro.\n", "");
    let fell_back = fallback(resume(&removed, save(&base, SavePoint::AfterIntro)));
    assert_eq!(
        fell_back.reason,
        ResumeFallbackReason::LandmarkRemoved {
            landmark: ResumeLandmark::Line(LineId::new("l_intro").expect("id")),
        }
    );

    let moved = compile_with(
        &BASE.replacen("> @l_intro\n  Intro.\n", "", 1),
        &format!("{OTHER}\n:: moved\n> @l_intro\n  Intro.\n-> END\n"),
        "0.0.1",
    );
    let fell_back = fallback(resume(&moved, save(&base, SavePoint::AfterIntro)));
    assert!(matches!(
        fell_back.reason,
        ResumeFallbackReason::LandmarkMoved { .. }
    ));

    let renamed = compile(&BASE.replacen(":: work\n", ":: labour\n", 1).replacen(
        "-> work\n",
        "-> labour\n",
        1,
    ));
    let fell_back = fallback(resume(&renamed, save(&base, SavePoint::InWorkBlock)));
    assert_eq!(fell_back.reason, ResumeFallbackReason::BlockRemoved);
    assert!(!fell_back.block_available);
    assert_eq!(fell_back.block_id.as_str(), "work");
    assert!(fell_back.restart_block(&renamed).is_err());
}

#[test]
fn split_prompt_is_ambiguous_rather_than_guessed() {
    let base = compile(BASE);
    let split = edited(
        "  ? @c_leave\n    Leave.\n    -> END\n",
        "> @l_prompt_two\n  Or else?\n  ? @c_leave\n    Leave.\n    -> END\n",
    );
    let fell_back = fallback(resume(&split, save(&base, SavePoint::PendingPrompt)));
    assert!(matches!(
        fell_back.reason,
        ResumeFallbackReason::LandmarkAmbiguous { .. }
    ));
}

#[test]
fn host_policy_limits_how_far_a_save_may_move() {
    let base = compile(BASE);
    let text = edited("  Second.\n", "  Second!\n");
    let saved = save(&base, SavePoint::AfterIntro);
    let refused =
        resume_session(&text, saved.clone(), &ResumeOptions::exact_only()).expect("not an error");
    assert_eq!(
        fallback(refused).reason,
        ResumeFallbackReason::LevelRefused {
            required: ResumeLevel::TextEdited,
            allowed: ResumeLevel::Exact,
        }
    );
    let allowed = resume_session(
        &text,
        saved,
        &ResumeOptions::new().with_max_level(ResumeLevel::TextEdited),
    )
    .expect("not an error");
    assert_eq!(resumed(allowed).1.level, ResumeLevel::TextEdited);
}

#[test]
fn ended_saves_stay_ended_and_other_assets_are_rejected() {
    let base = compile(BASE);
    let patched = edited("  Second.\n", "  Second!\n");
    let ended = save(&base, SavePoint::Ended);
    assert_eq!(ended.deferred_effects.len(), 1, "delivered with the end");
    let (session, _) = resumed(resume(&patched, ended.clone()));
    assert!(session.is_ended());

    // A fallback of an ended save hands back nothing: the end already
    // delivered its deferred effects.
    let refused = fallback(
        resume_session(&patched, ended.clone(), &ResumeOptions::exact_only())
            .expect("not an error"),
    );
    let renamed = edited(":: start default\n", ":: begin default\n");
    let removed = fallback(resume(&renamed, ended));
    assert!(matches!(
        refused.reason,
        ResumeFallbackReason::LevelRefused { .. }
    ));
    assert_eq!(removed.reason, ResumeFallbackReason::BlockRemoved);
    for fell_back in [refused, removed] {
        assert!(fell_back.ended);
        assert!(fell_back.deferred_effects.is_empty());
        assert!(fell_back.pending_effect.is_none());
    }
    let fell_back = fallback(resume(&renamed, save(&base, SavePoint::PendingPrompt)));
    assert!(!fell_back.ended);
    assert_eq!(fell_back.deferred_effects.len(), 1);

    let other_asset = {
        let report = compile_inputs(
            [CompileInput::new("dialogue/a.recite", BASE)],
            CompileOptions::new(
                CompilerVersion::new("0.0.1").expect("compiler"),
                CompiledAssetId::new("dialogue/other.recitec").expect("asset id"),
                SourceMapId::new("dialogue/other.recitec.map").expect("map id"),
                SchemaFingerprint::NoSchema,
            ),
        )
        .expect("compiles");
        report.asset.expect("asset").dialogue
    };
    assert!(matches!(
        resume_session(
            &other_asset,
            save(&base, SavePoint::AfterIntro),
            &ResumeOptions::new()
        ),
        Err(DialogueError::AssetMismatch { .. })
    ));
}

#[test]
fn messagepack_resume_is_deterministic() {
    let base = compile(BASE);
    let patched = edited("> @l_intro\n", "> @l_new_first\n  New first.\n> @l_intro\n");
    let bytes =
        encode_session_messagepack(&base, &drive(&base, SavePoint::PendingGrant)).expect("encodes");
    let first =
        resume_session_messagepack(&patched, &bytes, &ResumeOptions::new()).expect("resumes");
    let second =
        resume_session_messagepack(&patched, &bytes, &ResumeOptions::new()).expect("resumes");
    assert_eq!(first, second);
    let (mut session, report) = resumed(first);
    assert_eq!(report.level, ResumeLevel::Relocated);
    let saved_id = session.pending_effect().expect("pending").id.clone();
    let DialogueEvent::Effect(effect) = step(&patched, &mut session) else {
        panic!("re-emits the pending effect");
    };
    assert_eq!(effect.id, saved_id);
    acknowledge_effect(&mut session, saved_id, EffectAck::Completed).expect("acks original id");
}
