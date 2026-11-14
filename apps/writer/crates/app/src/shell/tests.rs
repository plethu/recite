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
