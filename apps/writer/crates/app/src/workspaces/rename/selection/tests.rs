use super::*;
use crate::navigation::worker::Job;
use crate::test_support as support;
use crate::workspaces::rename::inline::Stage;
use tokio::sync::mpsc;

const SOURCE: &str = ":: start default\n-> old\n:: old\n-> END\n";

/// A project whose beat `old` is selected in Script, with navigation running.
fn selected_old() -> (
    tempfile::TempDir,
    Writer,
    mpsc::UnboundedReceiver<Job>,
    tokio::runtime::Runtime,
) {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.inspect("old".into());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    (directory, writer, jobs, runtime)
}
fn request(writer: &mut Writer) -> BeatChoice {
    writer.choose_beat_to_rename();
    let Some(Request::Rename(request)) = writer.pending_choice.take() else {
        panic!("rename choice")
    };
    request
}

#[test]
fn the_chosen_beat_becomes_a_field_on_its_heading_once_script_selects_it() {
    let (_directory, mut writer, mut jobs, _runtime) = selected_old();
    let choice = request(&mut writer);
    assert_eq!(
        choice
            .choices
            .iter()
            .map(|choice| choice.value.as_str())
            .collect::<Vec<_>>(),
        ["start", "old"]
    );
    // The selected beat is renamed in place at once.
    choice.apply(&mut writer, "old".into());
    let inline = writer.inline_rename().unwrap();
    assert_eq!(
        (inline.beat.as_str(), inline.place),
        ("old", Place::Heading)
    );
    assert_eq!(inline.stage, Stage::Editing);
    writer.cancel_inline_rename();
    // Another beat is selected first, then its heading becomes the field.
    request(&mut writer).apply(&mut writer, "start".into());
    assert!(writer.inline_rename().is_none());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert_eq!(
        writer.session.model().selected_block().unwrap().as_deref(),
        Some("start")
    );
    let inline = writer.inline_rename().unwrap();
    assert_eq!(
        (inline.beat.as_str(), inline.place),
        ("start", Place::Heading)
    );
    assert!(writer.workspaces.rename.chosen.is_none());
    assert!(writer.workspaces.screen == Screen::Write);
}

#[test]
fn rename_beat_with_no_beat_to_rename_in_place_asks_which_beat_rather_than_opening_a_form() {
    let (_directory, mut writer, _jobs, _runtime) = selected_old();
    writer.workspaces.screen = Screen::Build;
    writer.rename_beat();
    assert!(matches!(writer.pending_choice, Some(Request::Rename(_))));
    assert!(writer.workspaces.screen == Screen::Build);
    // Choosing returns to writing and renames the beat there.
    let Some(Request::Rename(choice)) = writer.pending_choice.take() else {
        unreachable!()
    };
    choice.apply(&mut writer, "old".into());
    assert!(writer.workspaces.screen == Screen::Write);
    assert_eq!(writer.inline_rename().unwrap().beat, "old");
}

#[test]
fn stale_or_undeclared_beat_choices_start_no_rename() {
    for change in 0..4 {
        let (_directory, mut writer, _jobs, _runtime) = selected_old();
        let choice = request(&mut writer);
        match change {
            0 => writer.change(|model| {
                model.select(recite_writer_model::View::Source)?;
                model.set_draft(format!("{SOURCE}\n# newer source\n"));
                model.apply()
            }),
            1 => writer.phase = Phase::ConfirmClose,
            2 => writer.rename_beat(),
            _ => {}
        }
        let open = writer.inline_rename().cloned();
        let before = writer.session.model().recovery();
        choice.apply(
            &mut writer,
            if change == 3 { "not_captured" } else { "start" }.into(),
        );
        assert_eq!(writer.inline_rename().cloned(), open);
        assert!(writer.workspaces.rename.chosen.is_none());
        assert_eq!(writer.session.model().recovery(), before);
        assert!(!writer.navigation.busy() && !writer.project_jobs_busy());
    }
}
