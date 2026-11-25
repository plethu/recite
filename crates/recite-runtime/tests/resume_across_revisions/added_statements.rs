//! Patches that add statements around a saved point.

use super::*;

#[test]
fn statements_between_the_landmark_and_the_saved_point_must_be_unchanged() {
    let base = compile(BASE);
    // A line between the landmark and a waiting blocking effect, or the branch
    // the save is inside, could be a line that followed the saved point
    // before the patch, so it is not skipped.
    let before_grant = edited(
        "  ! blocking grant(map)\n",
        "  > @l_new\n    New.\n  ! blocking grant(map)\n",
    );
    let before_match = edited(":match mood()\n", "> @l_new\n  New.\n:match mood()\n");
    for (patched, point) in [
        (&before_grant, SavePoint::PendingGrant),
        (&before_match, SavePoint::InsideMatch),
    ] {
        assert!(
            matches!(
                fallback(resume(patched, save(&base, point))).reason,
                ResumeFallbackReason::StatementsChanged { .. }
            ),
            "{point:?}"
        );
    }

    // Removing an effect that has a twin further on must not slide the save
    // past the lines between them to the twin.
    let twins = concat!(
        ":: start default\n",
        "> @l_a\n  A.\n",
        "! immediate play_sound(door)\n",
        "> @l_b\n  B.\n",
        "! immediate play_sound(door)\n",
        "> @l_d\n  D.\n",
        "-> END\n",
    );
    let twins_base = compile(twins);
    let mut session = recite_runtime::start_scene(&twins_base, None).expect("starts");
    step(&twins_base, &mut session);
    step(&twins_base, &mut session);
    let saved = recite_runtime::snapshot::snapshot_session(&twins_base, &session).expect("saves");
    let removed = compile(&twins.replacen("! immediate play_sound(door)\n", "", 1));
    assert!(matches!(
        fallback(resume(&removed, saved)).reason,
        ResumeFallbackReason::StatementsChanged { .. }
    ));
}

#[test]
fn a_structural_change_in_the_block_evaluates_a_waiting_prompt_again() {
    let base = compile(BASE);
    let saved = save(&base, SavePoint::PendingPrompt);
    let tightened = edited("  ? @c_leave\n", "  ? requires=(banned(player)) @c_leave\n");
    let (_, report) = resumed(resume(&tightened, saved.clone()));
    assert_eq!(report.prompt, PromptResume::Reevaluated);
    assert_eq!(report.level, ResumeLevel::Relocated);

    let reworded = edited("    Leave.\n", "    Go.\n");
    let (_, report) = resumed(resume(&reworded, saved));
    assert_eq!(report.prompt, PromptResume::Kept);
    assert_eq!(report.level, ResumeLevel::TextEdited);
}

#[test]
fn a_change_in_a_branch_without_lines_names_the_line_before_the_branch() {
    let source = concat!(
        ":: start default\n",
        "> @l_a\n  A.\n",
        ":if trusts(player)\n",
        "  ! immediate fx(one)\n",
        "  > @l_b\n    B.\n",
        "-> END\n",
    );
    let base = compile(source);
    let mut session = recite_runtime::start_scene(&base, None).expect("starts");
    step(&base, &mut session);
    assert!(matches!(
        step(&base, &mut session),
        DialogueEvent::Effect(_)
    ));
    let saved = recite_runtime::snapshot::snapshot_session(&base, &session).expect("saves");
    let patched = compile(&source.replace("fx(one)", "fx(two)"));
    assert_eq!(
        fallback(resume(&patched, saved)).reason,
        ResumeFallbackReason::StatementsChanged {
            landmark: Some(ResumeLandmark::Line(LineId::new("l_a").expect("id"))),
        }
    );
}

#[test]
fn a_line_that_becomes_a_prompt_is_removed_rather_than_moved() {
    let base = compile(BASE);
    let became_prompt = edited(
        "> @l_second\n  Second.\n",
        "> @l_second\n  Second.\n  ? @c_next\n    Next.\n    -> work\n",
    );
    let saved_after_line = {
        let mut session = drive(&base, SavePoint::AfterChime);
        step(&base, &mut session);
        recite_runtime::snapshot::snapshot_session(&base, &session).expect("saves")
    };
    assert_eq!(
        fallback(resume(&became_prompt, saved_after_line)).reason,
        ResumeFallbackReason::LandmarkRemoved {
            landmark: ResumeLandmark::Line(LineId::new("l_second").expect("id")),
        }
    );

    let became_line = edited(
        "  ? @c_work\n    Work.\n    -> work\n  ? @c_leave\n    Leave.\n    -> END\n",
        "-> END\n",
    );
    assert!(matches!(
        fallback(resume(&became_line, save(&base, SavePoint::PendingPrompt))).reason,
        ResumeFallbackReason::LandmarkRemoved { .. }
    ));
}
