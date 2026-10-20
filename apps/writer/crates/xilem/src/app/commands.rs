use super::*;
use recite_config::WriterCommand;
use recite_writer_model::Workbench;

impl App {
    pub(super) fn run_command(&mut self, command: WriterCommand) {
        if self
            .writer
            .as_ref()
            .is_some_and(|writer| writer.phase != Phase::Writing)
        {
            self.status = "Choose how to close Writer before running another command.".into();
            return;
        }
        match command {
            WriterCommand::Settings => self.show_settings(),
            WriterCommand::OpenProject => {
                self.show_welcome = true;
                self.settings_open = false;
            }
            WriterCommand::Close => self.request_close(),
            _ if self.settings_open || self.show_welcome => {
                self.status = "Return to your document to run this command.".into()
            }
            _ => {
                let Some(writer) = &mut self.writer else {
                    return;
                };
                match command {
                    WriterCommand::Save => writer.save(),
                    WriterCommand::SaveAll => writer.save_all(),
                    WriterCommand::Source => writer.open_editor(),
                    WriterCommand::Apply => writer.navigate(crate::navigation::Target::Edit(
                        recite_writer_model::EditOperation::Apply,
                    )),
                    WriterCommand::AddBeat => writer.change(Workbench::add_beat),
                    WriterCommand::AddLine => writer.change(Workbench::add_line),
                    WriterCommand::AddReply => writer.change(Workbench::add_choice),
                    WriterCommand::Preview => writer.start_trial(),
                    _ => {
                        writer.status = format!(
                            "{} is not available in this frontend yet.",
                            recite_writer_session::commands::label(command, false)
                        )
                    }
                }
            }
        }
    }
}
