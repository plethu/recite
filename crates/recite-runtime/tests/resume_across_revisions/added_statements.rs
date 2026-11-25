//! Patches that add statements around a saved point.

use super::*;

#[test]
fn lines_added_before_the_saved_point_are_skipped_like_earlier_lines() {
    let base = compile(BASE);
    // Between the landmark and a waiting blocking effect.
    let before_grant = edited(
        "  ! blocking grant(map)\n",
        "  > @l_new\n    New.\n  ! blocking grant(map)\n",
    );
    let (session, report) = resumed(resume(&before_grant, save(&base, SavePoint::PendingGrant)));
    assert_eq!(report.level, ResumeLevel::Relocated);
    let mut session = assert_saves_again(&before_grant, session);
    let DialogueEvent::Effect(effect) = step(&before_grant, &mut session) else {
        panic!("the waiting effect is emitted again");
    };
    assert_eq!(effect.function, "grant");
    acknowledge_effect(&mut session, effect.id, EffectAck::Completed).expect("acks");
    let DialogueEvent::Line(line) = step(&before_grant, &mut session) else {
        panic!("the added line precedes the saved point");
    };
    assert_eq!(line.id.as_str(), "l_after_grant");

    // Between the landmark and the branch the save is inside.
    let before_match = edited(":match mood()\n", "> @l_new\n  New.\n:match mood()\n");
    for point in [SavePoint::InsideIf, SavePoint::InsideMatch] {
        let (session, report) = resumed(resume(&before_match, save(&base, point)));
        assert_eq!(report.level, ResumeLevel::Relocated, "{point:?}");
        assert_continues_like_fresh(&before_match, point, session);
    }

    // A new prompt is not skipped: its choices could lead anywhere.
    let prompt = edited(
        "  ! blocking grant(map)\n",
        "  > @l_ask\n    Ask?\n    ? @c_yes\n      Yes.\n      -> work\n  ! blocking grant(map)\n",
    );
    assert!(matches!(
        fallback(resume(&prompt, save(&base, SavePoint::PendingGrant))).reason,
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
