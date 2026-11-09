use super::*;
pub use crate::{
    attention::{DocumentMenu, PaneMotion},
    map::Workspace,
    navigation::Target,
    workspaces::Open,
};
use recite_config::WriterCommand;
use recite_writer_model::{EditOperation, StructuralCommand};

impl Shell {
    pub fn run_command(&mut self, command: WriterCommand) {
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
        if let Some(writer) = &mut self.writer {
            writer.document_menu = DocumentMenu::Closed;
        }
        match (&mut self.writer, command) {
            (_, WriterCommand::Commands) => self.open_picker(picker::Search::Commands),
            (_, WriterCommand::GoTo | WriterCommand::Find) => {
                self.open_picker(picker::Search::GoTo)
            }
            (_, WriterCommand::Settings) => self.show_settings(),
            (_, WriterCommand::OpenProject) => {
                self.show_welcome = true;
                self.settings_open = false;
            }
            (_, WriterCommand::Close) => self.request_close(),
            (_, _) if self.settings_open || self.show_welcome => {
                self.status = "Return to your document to run this command.".into()
            }
            (Some(writer), WriterCommand::Script) => writer.show_workspace(Workspace::Script),
            (Some(writer), WriterCommand::Map) => writer.show_workspace(Workspace::Map),
            (Some(writer), WriterCommand::Split) => writer.show_workspace(Workspace::Split),
            (Some(writer), WriterCommand::Focus) => writer.toggle_focus(),
            (Some(writer), WriterCommand::Localise) => writer.open_localisation(),
            (Some(writer), WriterCommand::Declarations) => {
                writer.open_project_workspace(Open::Declarations)
            }
            (Some(writer), WriterCommand::Build) => writer.open_project_workspace(Open::Build),
            (Some(writer), WriterCommand::Rename) => writer.open_project_workspace(Open::Rename),
            (Some(writer), WriterCommand::NextMatch | WriterCommand::PreviousMatch) => {
                writer.repeat_lookup(command == WriterCommand::NextMatch)
            }
            (Some(writer), WriterCommand::PaneLeft) => writer.focus_pane(PaneMotion::Left),
            (Some(writer), WriterCommand::PaneRight) => writer.focus_pane(PaneMotion::Right),
            (Some(writer), WriterCommand::PaneUp) => writer.focus_pane(PaneMotion::Up),
            (Some(writer), WriterCommand::PaneDown) => writer.focus_pane(PaneMotion::Down),
            (Some(writer), WriterCommand::Save) => writer.save(),
            (Some(writer), WriterCommand::SaveAll) => writer.save_all(),
            (Some(writer), WriterCommand::Source) => writer.open_editor(),
            (Some(writer), WriterCommand::Apply) => {
                writer.navigate(Target::Edit(EditOperation::Apply))
            }
            (Some(writer), WriterCommand::Undo | WriterCommand::Redo) => {
                writer.undo_redo(command == WriterCommand::Redo)
            }
            (Some(writer), WriterCommand::AddBeat) => {
                writer.navigate(Target::Structure(StructuralCommand::AddBeat))
            }
            (Some(writer), WriterCommand::AddLine) => {
                writer.navigate(Target::Structure(StructuralCommand::AddLine))
            }
            (Some(writer), WriterCommand::AddReply) => {
                writer.navigate(Target::Structure(StructuralCommand::AddReply))
            }
            (Some(writer), WriterCommand::Preview) => writer.start_trial(),
            (Some(writer), WriterCommand::Back | WriterCommand::Forward) => {
                writer.history_step(command == WriterCommand::Forward)
            }
            (None, _) => (),
        }
    }
}

#[cfg(test)]
mod tests;
