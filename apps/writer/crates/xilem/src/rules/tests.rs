use super::*;
use crate::navigation::{self, Target};
use crate::tests::support;
use recite_writer_model::EditOperation;
use recite_writer_session::location::{Location, Screen};
use std::sync::Arc;
#[path = "bounds/tests.rs"]
mod bounds;
#[path = "lifecycle/tests.rs"]
mod lifecycle;
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
        .rules
        .as_ref()
        .ok_or("No reply session")?
        .controls
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
    let revision = writer.rules.as_ref().unwrap().controls.revision;
    writer.edit_rule_argument(target.clone(), "5".into());
    assert!(writer.session.model().draft().contains("standing(5)"));
    assert_eq!(writer.session.model().document().source(), SOURCE);
    writer.navigate(Target::Edit(EditOperation::Apply));
    assert!(!writer.rules_editable());
    writer.edit_rule_argument(target.clone(), "99".into());
    support::finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(writer.rules_editable());
    assert_eq!(writer.rules.as_ref().unwrap().controls.revision, revision);
    assert_eq!(
        target
            .get(&writer.rules.as_ref().unwrap().projection)
            .unwrap()
            .value,
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
        target
            .get(&writer.rules.as_ref().unwrap().projection)
            .unwrap()
            .value,
        "5"
    );
    assert_ne!(writer.rules.as_ref().unwrap().controls.revision, revision);
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
    writer.rules.as_mut().unwrap().projection.effects[0].arguments[0].choices =
        vec!["gate_latch".into(), "bell".into()];
    writer.pick_rule_choice(selection::Target::Argument(target));
    let request = match writer.pending_choice.take().unwrap() {
        crate::choices::Request::Rules(request) => request,
        _ => panic!("expected a rule request"),
    };
    assert!(request.is_current(writer));
    writer.remove_effect(0);
    let before = writer.session.model().recovery();
    assert!(!request.apply(writer, selection::Choice::Argument("bell".into())));
    assert_eq!(writer.session.model().recovery(), before);
    assert_eq!(
        writer.rules.as_ref().unwrap().projection.effects[0].function,
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
            target
                .get(&writer.rules.as_ref().unwrap().projection)
                .unwrap()
                .value,
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
