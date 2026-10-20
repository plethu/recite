use super::*;
use crate::attention::PaneMotion;
use recite_config::WriterCommand;
use recite_writer_model::StructuralCommand;

impl App {
    pub(super) fn run_command(&mut self, command: WriterCommand) {
        if self.picker.is_some() {
            match command {
                WriterCommand::Commands => self.open_picker(picker::Search::Commands),
                WriterCommand::GoTo | WriterCommand::Find => self.open_picker(picker::Search::GoTo),
                WriterCommand::NextMatch => self.step_picker(true),
                WriterCommand::PreviousMatch => self.step_picker(false),
                WriterCommand::Close => self.dismiss_picker(),
                _ => {}
            }
            return;
        }
        if self
            .writer
            .as_ref()
            .is_some_and(|writer| writer.phase != Phase::Writing)
        {
            self.status = "Choose how to close Writer before running another command.".into();
            return;
        }
        match command {
            WriterCommand::Commands => self.open_picker(picker::Search::Commands),
            WriterCommand::GoTo | WriterCommand::Find => self.open_picker(picker::Search::GoTo),
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
                    WriterCommand::Script => writer.show_workspace(crate::map::Workspace::Script),
                    WriterCommand::Map => writer.show_workspace(crate::map::Workspace::Map),
                    WriterCommand::Split => writer.show_workspace(crate::map::Workspace::Split),
                    WriterCommand::Focus => writer.toggle_focus(),
                    WriterCommand::NextMatch | WriterCommand::PreviousMatch => {
                        writer.repeat_lookup(command == WriterCommand::NextMatch)
                    }
                    WriterCommand::PaneLeft => writer.focus_pane(PaneMotion::Left),
                    WriterCommand::PaneRight => writer.focus_pane(PaneMotion::Right),
                    WriterCommand::PaneUp => writer.focus_pane(PaneMotion::Up),
                    WriterCommand::PaneDown => writer.focus_pane(PaneMotion::Down),
                    WriterCommand::Save => writer.save(),
                    WriterCommand::SaveAll => writer.save_all(),
                    WriterCommand::Source => writer.open_editor(),
                    WriterCommand::Apply => writer.navigate(crate::navigation::Target::Edit(
                        recite_writer_model::EditOperation::Apply,
                    )),
                    WriterCommand::Undo | WriterCommand::Redo => writer.navigate(
                        crate::navigation::Target::Edit(if command == WriterCommand::Undo {
                            recite_writer_model::EditOperation::Undo
                        } else {
                            recite_writer_model::EditOperation::Redo
                        }),
                    ),
                    WriterCommand::AddBeat | WriterCommand::AddLine | WriterCommand::AddReply => {
                        writer.navigate(crate::navigation::Target::Structure(match command {
                            WriterCommand::AddBeat => StructuralCommand::AddBeat,
                            WriterCommand::AddLine => StructuralCommand::AddLine,
                            _ => StructuralCommand::AddReply,
                        }))
                    }
                    WriterCommand::Preview => writer.start_trial(),
                    WriterCommand::Back | WriterCommand::Forward => {
                        writer.history_step(command == WriterCommand::Forward)
                    }
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
