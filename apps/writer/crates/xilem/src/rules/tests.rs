use super::*;
use crate::navigation::{self, Target};
use crate::tests::support;
use recite_writer_model::EditOperation;
use recite_writer_session::location::{Location, Screen};
use std::sync::Arc;
#[path = "bounds/tests.rs"]
mod bounds;
#[path = "visual/tests.rs"]
mod visual;
const SOURCE: &str = "# Keep this comment.\n:: start default\n? reply@22222222222222222222 requires=(standing(3))\n  Let me through.\n  -> accepted\n:: accepted\n! immediate play_sfx(gate_latch)\n# Keep effect spacing.\n! blocking mark_map(floodgate)\n-> END\n";
struct Fixture {
    directory: tempfile::TempDir,
    writer: Writer,
    jobs: tokio::sync::mpsc::UnboundedReceiver<navigation::worker::Job>,
    _runtime: tokio::runtime::Runtime,
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    open_fixture(SOURCE)
}
fn open_fixture(source: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let directory = support::project(source)?;
    let mut writer = support::open(&directory)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (sender, mut jobs) = tokio::sync::mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.navigate(Target::Location(Location {
        screen: Screen::Rules,
        passage: Some("22222222222222222222".into()),
        ..Location::default()
    }));
    support::finish_navigation(&mut writer, &mut jobs)?;
    Ok(Fixture {
        directory,
        writer,
        jobs,
        _runtime: runtime,
    })
}
fn first_argument(writer: &Writer) -> Result<ArgumentTarget, Box<dyn std::error::Error>> {
    writer
        .rule_controls
        .rows
        .iter()
        .find_map(|row| match row {
            Row::Argument(target) => Some(target.clone()),
            _ => None,
        })
        .ok_or_else(|| "No rule argument".into())
}

#[test]
fn apply_reprojects_ranges_preserves_control_identity_and_typed_edits_stay_locked_until_ready() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = first_argument(writer).unwrap();
    let revision = writer.rule_controls.revision;
    writer.edit_rule_argument(target.clone(), "5".into());
    assert!(writer.session.model().draft().contains("standing(5)"));
    assert_eq!(writer.session.model().document().source(), SOURCE);
    writer.navigate(Target::Edit(EditOperation::Apply));
    assert!(!writer.rules_editable());
    writer.edit_rule_argument(target.clone(), "99".into());
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules_editable());
    assert_eq!(writer.rule_controls.revision, revision);
    assert_eq!(
        target.get(writer.rules.as_ref().unwrap()).unwrap().value,
        "5"
    );
    assert_eq!(writer.current_location().screen, Screen::Rules);
    assert_eq!(
        writer.current_location().passage.as_deref(),
        Some("22222222222222222222")
    );
    writer.edit_rule_argument(target.clone(), "7".into());
    writer.navigate(Target::Edit(EditOperation::Apply));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("standing(7)")
    );
    writer.navigate(Target::Edit(EditOperation::Undo));
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules_editable());
    assert_eq!(
        target.get(writer.rules.as_ref().unwrap()).unwrap().value,
        "5"
    );
    assert_ne!(writer.rule_controls.revision, revision);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("# Keep effect spacing.")
    );
}

#[test]
fn invalid_rule_save_keeps_recovery_then_corrected_save_refreshes_controls() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = first_argument(writer).unwrap();
    let (sender, mut saves) = tokio::sync::mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.edit_rule_argument(target.clone(), "-".into());
    writer.save();
    support::finish_work(writer, &mut saves).unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    assert!(writer.session.model().draft().contains("standing(-)"));
    assert!(writer.rules_editable());
    assert!(!writer.navigation.busy());
    writer.edit_rule_argument(target, "9".into());
    writer.save();
    support::finish_work(writer, &mut saves).unwrap();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules_editable());
    assert!(!writer.session.dirty());
    assert!(
        std::fs::read_to_string(fixture.directory.path().join("scene.recite"))
            .unwrap()
            .contains("standing(9)")
    );
    assert_eq!(writer.status, "Saved");
}

#[test]
fn stale_picker_receipt_cannot_edit_a_different_effect_after_removal() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = ArgumentTarget {
        call: CallTarget::Effect(0),
        index: 0,
    };
    writer.rules.as_mut().unwrap().effects[0].arguments[0].choices =
        vec!["gate_latch".into(), "bell".into()];
    writer.pick_rule_choice(selection::Target::Argument(target));
    let request = writer.rule_controls.pending_choice.take().unwrap();
    assert!(request.is_current(writer));
    writer.remove_effect(0);
    let before = writer.session.model().recovery();
    assert!(!request.apply(writer, selection::Choice::Argument("bell".into())));
    assert_eq!(writer.session.model().recovery(), before);
    assert_eq!(
        writer.rules.as_ref().unwrap().effects[0].function,
        "mark_map"
    );
    assert!(writer.status.contains("Reopen"));
}

#[test]
fn deferred_save_waits_for_apply_retirement_and_refreshes_even_after_a_disk_conflict() {
    for conflict in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        let (sender, mut saves) = tokio::sync::mpsc::unbounded_channel();
        writer.work.sender = Some(sender);
        let target = first_argument(writer).unwrap();
        writer.edit_rule_argument(target.clone(), "5".into());
        if conflict {
            std::fs::write(
                fixture.directory.path().join("scene.recite"),
                SOURCE.replace("Let me", "Please let me"),
            )
            .unwrap();
        }
        writer.navigate(Target::Edit(EditOperation::Apply));
        writer.save();
        let completed = fixture.jobs.try_recv().unwrap().execute();
        let request = completed.request.clone();
        drop(writer.accept_navigation(completed));
        assert!(!writer.work.busy());
        assert!(
            fixture.jobs.try_recv().is_err(),
            "deferred Save takes precedence over reprojection"
        );
        writer.navigation_retired(&request);
        support::finish_work(writer, &mut saves).unwrap();
        support::finish_navigation(writer, &mut fixture.jobs).unwrap();
        assert!(writer.rules_editable());
        assert_eq!(
            target.get(writer.rules.as_ref().unwrap()).unwrap().value,
            "5"
        );
        assert_eq!(writer.session.dirty(), conflict);
        if conflict {
            assert!(writer.status.contains("changed on disk"));
        } else {
            assert_eq!(writer.status, "Saved");
        }
    }
}

#[test]
fn editing_reply_text_applies_checked_rule_draft_and_exits_source_backing_view() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    let target = first_argument(writer).unwrap();
    writer.edit_rule_argument(target, "5".into());
    writer.edit_reply_text();
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules.is_none());
    assert_eq!(
        writer.session.model().view(),
        &recite_writer_model::View::Passage("22222222222222222222".into())
    );
    assert_eq!(writer.current_location().screen, Screen::Write);
    assert_eq!(writer.session.model().draft(), "Let me through.");
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("standing(5)")
    );
}

#[test]
fn structural_authoring_and_reload_retire_rule_presentation_and_its_picker_receipts() {
    for reload in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.pick_rule_choice(selection::Target::AddCondition(None));
        let receipt = writer.rule_controls.pending_choice.take().unwrap();
        if reload {
            writer.reload();
        } else {
            writer.change(recite_writer_model::Workbench::add_beat);
        }
        assert!(writer.rules.is_none());
        assert!(!receipt.is_current(writer));
        assert_eq!(writer.current_location().screen, Screen::Write);
    }
}

#[test]
fn undo_removing_the_open_reply_retires_controls_and_preserves_redo() {
    let directory =
        support::project(":: start default\n> line@11111111111111111111\n  Begin.\n-> END\n")
            .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = tokio::sync::mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.change(recite_writer_model::Workbench::add_choice);
    let recite_writer_model::View::Passage(passage) = writer.session.model().view().clone() else {
        panic!("new reply must be selected")
    };
    writer.navigate(Target::Location(Location {
        screen: Screen::Rules,
        passage: Some(passage),
        ..Location::default()
    }));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.rules.is_some());
    writer.navigate(Target::Edit(EditOperation::Undo));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.rules.is_none());
    assert_eq!(writer.current_location().screen, Screen::Write);
    assert!(writer.current_location().source);
    assert!(!writer.status.is_empty());
    assert!(
        !writer
            .session
            .model()
            .document()
            .source()
            .contains("? reply")
    );
    assert!(writer.session.model().document().can_redo());
    writer.navigate(Target::Edit(EditOperation::Redo));
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(writer.session.model().document().source().contains("? "));
}
