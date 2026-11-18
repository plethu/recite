//! Which commands the command picker offers as available for the current writer and window.
use super::*;

impl Shell {
    pub fn command_enabled(&self, command: WriterCommand) -> bool {
        use WriterCommand as C;
        let Some(writer) = &self.writer else {
            return matches!(command, C::OpenProject | C::Settings | C::Close);
        };
        if writer.phase != Phase::Writing {
            return false;
        }
        let idle = !writer.work.busy() && !writer.navigation.busy() && !writer.session.read_only();
        match command {
            C::Commands | C::GoTo | C::Find | C::OpenProject | C::Settings | C::Close => true,
            C::Save | C::SaveAll => {
                idle && !writer.localisation.saving()
                    && (command != C::SaveAll || !writer.localisation.working())
                    && !writer.source_repair_pending()
                    && writer.session.project().is_some()
            }
            C::Source => idle && !writer.external.busy() && writer.session.project().is_some(),
            C::Apply => idle && writer.session.model().has_draft(),
            C::Undo => {
                idle && (writer.session.model().has_draft()
                    || writer.session.model().document().can_undo()
                    || writer.has_project_history(false))
            }
            C::Redo => {
                idle && (writer.session.model().document().can_redo()
                    || writer.has_project_history(true))
            }
            C::Script | C::Map | C::Split | C::AddBeat | C::Focus => idle,
            C::Sidebar => !writer.attention.is_focused(),
            C::CopyLink => !writer.project_action_blocked(),
            C::PaneLeft | C::PaneRight | C::PaneUp | C::PaneDown => true,
            C::NextMatch | C::PreviousMatch => idle,
            C::AddLine | C::AddReply => {
                idle && writer
                    .session
                    .model()
                    .selected_block()
                    .ok()
                    .flatten()
                    .is_some()
            }
            C::Preview => idle,
            C::Declarations | C::Build | C::Rename => idle && writer.session.project().is_some(),
            C::Localise => !writer.project_action_blocked() && writer.session.project().is_some(),
            C::Back | C::Forward => {
                idle && writer
                    .navigation
                    .history
                    .step(command == C::Forward)
                    .is_some()
            }
        }
    }
}
