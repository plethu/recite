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
