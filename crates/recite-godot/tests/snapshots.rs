mod support;

use recite_godot::ReciteDialogueDriver;

use recite_adapter::{AdapterErrorKind, RestoreOutcome};
use recite_runtime::resume::{ResumeFallbackReason, ResumeLevel, ResumeOptions};
use support::{assert_line, compile_asset, must_ok, must_ok_unit, must_resume, output_kinds};

#[test]
fn snapshots_restore_pending_prompt_and_present_it_again() {
    let asset = compile_asset(
        "dialogue/snapshot.recite",
        "dialogue/snapshot.recitec",
        concat!(
            ":: start default\n",
            "> @prompt\n",
            "  Save?\n",
            "  ? @continue\n",
            "    Continue.\n",
            "    -> after\n",
            ":: after\n",
            "> @after\n",
            "  Restored.\n",
            "-> END\n",
        ),
    );
    let mut driver = ReciteDialogueDriver::new();
    let outputs = must_ok(driver.start(&asset, None, None));
    assert_eq!(output_kinds(&outputs), ["prompt"]);

    let snapshot = match driver.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => panic!("snapshot should encode: {error}"),
    };
    must_ok_unit(driver.end_session());
    let restored_outputs = must_resume(driver.restore(&asset, &snapshot));
    assert_eq!(output_kinds(&restored_outputs), ["prompt"]);

    let outputs = must_ok(driver.select_choice("continue"));
    assert_eq!(output_kinds(&outputs), ["line", "end"]);
    assert_line(&outputs[0], "after", "Restored.");
}

#[test]
fn restore_resumes_patched_text_and_falls_back_when_ids_change() {
    let base = concat!(
        ":: start default\n",
        "! deferred remember(start)\n",
        "> @before\n",
        "  Choose.\n",
        "  ? @continue\n",
        "    Continue.\n",
        "    -> END\n",
    );
    let asset = compile_asset("dialogue/patch.recite", "dialogue/patch.recitec", base);
    let mut driver = ReciteDialogueDriver::new();
    must_ok(driver.start(&asset, None, None));
    let snapshot = driver.snapshot().expect("snapshot at prompt");
    must_ok_unit(driver.end_session());

    let reworded = compile_asset(
        "dialogue/patch.recite",
        "dialogue/patch.recitec",
        base.replace("    Continue.\n", "    Carry on.\n"),
    );
    match driver
        .restore_with_options(&reworded, &snapshot, None, &ResumeOptions::new())
        .expect("restore is not an error")
    {
        RestoreOutcome::Resumed { report, output } => {
            assert_eq!(report.level, ResumeLevel::TextEdited);
            assert_eq!(output_kinds(&output), ["prompt"]);
        }
        other => panic!("expected a resumed save, got {other:?}"),
    }
    must_ok_unit(driver.end_session());

    let reassigned = compile_asset(
        "dialogue/patch.recite",
        "dialogue/patch.recitec",
        base.replace("> @before", "> @asking")
            .replace("? @continue", "? @proceed"),
    );
    let RestoreOutcome::Fallback(fallback) = driver
        .restore(&reassigned, &snapshot)
        .expect("restore is not an error")
    else {
        panic!("a save whose prompt lost its choices falls back");
    };
    assert!(matches!(
        fallback.reason,
        ResumeFallbackReason::LandmarkRemoved { .. }
    ));
    assert_eq!(fallback.deferred_effects[0].function, "remember");
    assert!(!driver.has_active_session());

    // GDScript names the most a save may move.
    let RestoreOutcome::Fallback(fallback) = driver
        .restore_with_level(&reworded, &snapshot, None, Some("exact"))
        .expect("restore is not an error")
    else {
        panic!("an exact-only restore refuses an edited asset");
    };
    assert_eq!(
        fallback.reason,
        ResumeFallbackReason::LevelRefused {
            required: ResumeLevel::TextEdited,
            allowed: ResumeLevel::Exact,
        }
    );
    assert!(matches!(
        driver.restore_with_level(&reworded, &snapshot, None, Some("text_edited")),
        Ok(RestoreOutcome::Resumed { .. })
    ));
    must_ok_unit(driver.end_session());
    let error = driver
        .restore_with_level(&reworded, &snapshot, None, Some("bogus"))
        .expect_err("an unknown level is rejected");
    assert_eq!(error.kind(), AdapterErrorKind::Validation);
    assert!(!driver.has_active_session());
}
