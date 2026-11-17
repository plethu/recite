use super::*;
use crate::test_support;
use welcome::ProjectAction;

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";

#[test]
fn switching_to_projects_closes_settings_and_the_workspace_menu() {
    let mut shell = Shell::empty();
    shell.settings_open = true;
    shell.workspace_menu = true;
    shell.show_welcome = false;
    shell.toggle_projects();
    assert!(shell.show_welcome && !shell.settings_open && !shell.workspace_menu);
    shell.toggle_projects();
    assert!(!shell.show_welcome);
}

#[test]
fn menu_actions_close_the_workspace_menu_first() {
    let mut shell = Shell::empty();
    shell.workspace_menu = true;
    shell.settings_from_menu();
    assert!(shell.settings_open && !shell.workspace_menu);
    shell.toggle_workspace_menu();
    assert!(shell.workspace_menu);
    shell.close_settings();
    assert!(!shell.settings_open);
}

#[test]
fn project_form_edits_clear_a_reported_problem_and_typed_folders_replace_chosen_ones() {
    let mut shell = Shell::empty();
    shell.project_failed(ProjectAction::Open, "missing".into());
    shell.native_path = Some("/chosen".into());
    shell.edit_project_path("/typed".into());
    assert!(shell.welcome_problem.is_none() && shell.native_path.is_none());
    assert_eq!(shell.path, "/typed");
    shell.project_failed(ProjectAction::Create, "exists".into());
    shell.choose_project_action(true);
    assert!(shell.creating && shell.welcome_problem.is_none());
    shell.project_failed(ProjectAction::Create, "exists".into());
    shell.edit_project_name("Saga".into());
    assert!(shell.name == "Saga" && shell.welcome_problem.is_none());
}

#[test]
fn writer_requests_reach_the_shell_once() {
    let directory = test_support::project(SOURCE).unwrap();
    let writer = test_support::open(&directory).unwrap();
    let mut shell = Shell::with_writer(writer);
    shell.writer.as_mut().unwrap().request_settings();
    shell.apply_writer_requests();
    assert!(shell.settings_open());
    shell.close_settings();
    shell.apply_writer_requests();
    assert!(!shell.settings_open(), "a taken request does not repeat");
    shell
        .writer
        .as_mut()
        .unwrap()
        .request_search(Search::Commands);
    shell.apply_writer_requests();
    assert!(shell.picker.is_some());
    shell.writer.as_mut().unwrap().request_projects();
    shell.apply_writer_requests();
    assert!(shell.show_welcome() && !shell.settings_open());
}

#[test]
fn keep_editing_returns_from_a_close_prompt() {
    let directory = test_support::project(SOURCE).unwrap();
    let mut writer = test_support::open(&directory).unwrap();
    writer.phase = Phase::ConfirmClose;
    writer.keep_editing();
    assert!(writer.phase == Phase::Writing);
}

#[test]
fn the_configure_preview_slip_is_a_modal_that_escape_dismisses() {
    use keyboard::{Action, Generations, Invocation, Modal, Scope};
    let directory = test_support::project(SOURCE).unwrap();
    let mut shell = Shell::with_writer(test_support::open(&directory).unwrap());
    let shortcuts = shell.preferences.config.writer.shortcuts.clone();
    let modal = |shell: &Shell| {
        Scope::capture(shell)
            .context(&shortcuts, Generations::default(), ())
            .modal
    };
    assert_eq!(modal(&shell), None);
    shell.writer.as_mut().unwrap().configure_trial();
    assert_eq!(modal(&shell), Some(Modal::PreviewInputs));
    let captured = Scope::capture(&shell);
    assert!(shell.invoke(
        &captured,
        &shortcuts,
        Generations::default(),
        Invocation {
            action: Action::DismissModal(Modal::PreviewInputs),
            generation: Generations::default().command,
        },
    ));
    assert!(!shell.writer.as_ref().unwrap().configuring_trial());
    assert_eq!(modal(&shell), None);
}

#[test]
fn escape_cancels_a_beat_renamed_in_place() {
    use crate::workspaces::rename::inline::{Inline, Place, Stage};
    use keyboard::{Action, Generations, Invocation, Modal, Scope};
    let directory = test_support::project(SOURCE).unwrap();
    let mut shell = Shell::with_writer(test_support::open(&directory).unwrap());
    let shortcuts = shell.preferences.config.writer.shortcuts.clone();
    let modal = |shell: &Shell| {
        Scope::capture(shell)
            .context(&shortcuts, Generations::default(), ())
            .modal
    };
    shell.writer.as_mut().unwrap().workspaces.rename.inline = Some(Inline {
        beat: "start".into(),
        name: "begin".into(),
        stage: Stage::Editing,
        place: Place::Heading,
        anchor: None,
    });
    assert_eq!(modal(&shell), Some(Modal::Rename));
    let captured = Scope::capture(&shell);
    assert!(shell.invoke(
        &captured,
        &shortcuts,
        Generations::default(),
        Invocation {
            action: Action::DismissModal(Modal::Rename),
            generation: Generations::default().command,
        },
    ));
    assert!(shell.writer.as_ref().unwrap().inline_rename().is_none());
    assert_eq!(modal(&shell), None);
}

#[test]
fn an_opened_picker_takes_the_place_of_the_label_that_opened_it() {
    use keyboard::{Action, Generations, Invocation, Scope};
    let directory = test_support::project(SOURCE).unwrap();
    let mut shell = Shell::with_writer(test_support::open(&directory).unwrap());
    let shortcuts = shell.preferences.config.writer.shortcuts.clone();
    shell.open_picker(Search::GoTo);
    assert_eq!(shell.picker.as_ref().unwrap().placement, None);
    let captured = Scope::capture(&shell);
    assert!(shell.invoke(
        &captured,
        &shortcuts,
        Generations::default(),
        Invocation {
            action: Action::PlacePicker { x: 240, y: 320 },
            generation: Generations::default().command,
        },
    ));
    assert_eq!(shell.picker.as_ref().unwrap().placement, Some((240, 320)));
}

#[test]
fn the_configure_preview_slip_hangs_where_the_host_measured_its_action() {
    use keyboard::{Action, Generations, Invocation, Scope};
    let directory = test_support::project(SOURCE).unwrap();
    let mut shell = Shell::with_writer(test_support::open(&directory).unwrap());
    let shortcuts = shell.preferences.config.writer.shortcuts.clone();
    shell.writer.as_mut().unwrap().configure_trial();
    assert_eq!(shell.writer.as_ref().unwrap().trial.settings.anchor, None);
    let captured = Scope::capture(&shell);
    assert!(shell.invoke(
        &captured,
        &shortcuts,
        Generations::default(),
        Invocation {
            action: Action::PlaceInputsSlip { x: 135 },
            generation: Generations::default().command,
        },
    ));
    assert_eq!(
        shell.writer.as_ref().unwrap().trial.settings.anchor,
        Some(135)
    );
}

#[test]
fn rename_beat_from_commands_asks_which_beat_while_its_shortcut_renames_the_selected_one() {
    let directory = test_support::project(SOURCE).unwrap();
    let writer = test_support::open(&directory).unwrap();
    let mut shell = Shell::with_writer(writer);
    shell.open_picker(Search::Commands);
    shell.filter_picker("Rename beat".into());
    let index = shell
        .picker
        .as_ref()
        .unwrap()
        .matches
        .iter()
        .position(|&entry| {
            matches!(
                shell.picker.as_ref().unwrap().entries[entry].item,
                picker::Item::Command(recite_config::WriterCommand::Rename)
            )
        })
        .expect("Rename beat in Commands");
    shell.activate_picker(index);
    let picker = shell.picker.as_ref().expect("the beat picker");
    assert_eq!(picker.mode(), picker::Mode::RenameBeat);
    assert!(shell.writer.as_ref().unwrap().inline_rename().is_none());
}
