use super::*;
use crate::test_support as support;

#[test]
fn window_commands_remain_available_without_a_document_and_workspace_commands_keep_welcome_guarded()
{
    let mut app = Shell::empty();
    app.run_command(WriterCommand::Settings);
    assert!(app.settings_open);
    app.run_command(WriterCommand::OpenProject);
    assert!(app.show_welcome && !app.settings_open);
    app.run_command(WriterCommand::Close);
    assert!(app.closed);

    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = Shell::with_writer(support::open(&directory).unwrap());
    let before = app.writer.as_ref().unwrap().session.model().recovery();
    app.run_command(WriterCommand::OpenProject);
    app.run_command(WriterCommand::AddLine);
    assert_eq!(
        app.writer.as_ref().unwrap().session.model().recovery(),
        before
    );
    assert_eq!(app.status, "Return to your document to run this command.");
}

#[test]
fn closing_a_picker_dismisses_it_without_closing_the_live_document() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = Shell::with_writer(support::open(&directory).unwrap());
    app.run_command(WriterCommand::Commands);
    assert!(app.picker.is_some());
    app.run_command(WriterCommand::Close);
    assert!(app.picker.is_none());
    assert!(app.writer.as_ref().unwrap().phase == Phase::Writing);
}
