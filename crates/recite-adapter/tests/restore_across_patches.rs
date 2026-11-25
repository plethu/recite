#![cfg(test)]
//! The shared driver restores saves against patched assets.

use recite_adapter::{PreparedRestore, RestoreOutcome, SessionDriver};
use recite_runtime::{
    DialogueEvent, EmptyDialogueContext, LocaleResolution,
    resume::{ResumeFallbackReason, ResumeLevel, ResumeOptions},
};

mod support;
use support::{asset, request};

const BASE: &str = concat!(
    ":: start default\n",
    "! deferred remember(start)\n",
    "> @ffee6p8\n",
    "  Before.\n",
    "> @bqyvpqq\n",
    "  After.\n",
    "-> END\n",
);

/// A prepared session saved before its first batch; this short scene would
/// otherwise drain to its end in one batch.
fn saved_at_start() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let base = asset(BASE)?;
    let mut driver = SessionDriver::new();
    driver.prepare(request(&base))?;
    Ok(driver.snapshot()?)
}

#[test]
fn edited_text_resumes_with_a_report() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = saved_at_start()?;
    let patched = asset(&BASE.replace("  After.\n", "  Afterwards.\n"))?;
    let mut driver = SessionDriver::new();
    let RestoreOutcome::Resumed { report, output } = driver.restore(
        &patched,
        &snapshot,
        &ResumeOptions::new(),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?
    else {
        panic!("text edits resume");
    };
    assert_eq!(report.level, ResumeLevel::TextEdited);
    assert!(output.iter().any(|event| matches!(
        event,
        DialogueEvent::Line(line) if line.text == "Afterwards."
    )));
    Ok(())
}

#[test]
fn refused_or_unlocatable_saves_fall_back_without_a_session()
-> Result<(), Box<dyn std::error::Error>> {
    let snapshot = saved_at_start()?;
    let patched = asset(&BASE.replace("  After.\n", "  Afterwards.\n"))?;
    let mut driver = SessionDriver::new();
    let PreparedRestore::Fallback(fallback) =
        driver.prepare_restore(&patched, &snapshot, &ResumeOptions::exact_only())?
    else {
        panic!("exact-only refuses an edited asset");
    };
    assert!(matches!(
        fallback.reason,
        ResumeFallbackReason::LevelRefused { .. }
    ));
    assert!(!driver.has_active_session());

    let renamed = asset(&BASE.replace(":: start default\n", ":: opening default\n"))?;
    let RestoreOutcome::Fallback(fallback) = driver.restore(
        &renamed,
        &snapshot,
        &ResumeOptions::new(),
        &EmptyDialogueContext,
        LocaleResolution::new(),
    )?
    else {
        panic!("a removed block falls back");
    };
    assert_eq!(fallback.reason, ResumeFallbackReason::BlockRemoved);
    assert!(!driver.has_active_session());
    Ok(())
}
