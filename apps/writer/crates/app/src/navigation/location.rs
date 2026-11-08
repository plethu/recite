//! History and links describe the actual native destination, including retained catalogue context.
use super::*;
use crate::workspaces::Screen as WorkspaceScreen;
use recite_writer_session::location::Screen as LinkedScreen;

pub fn requests_editor(location: &Location) -> bool {
    location.source || location.view == Some(recite_config::WriterView::Source)
}
pub fn selection_location(location: &Location) -> Location {
    let mut selection = location.clone();
    if requests_editor(location) {
        selection.source = false;
        selection.view = Some(recite_config::WriterView::Script);
    }
    selection
}

impl Writer {
    pub fn copy_link(&mut self) {
        if self.phase != crate::Phase::Writing || self.project_action_blocked() {
            self.status = "Finish the current action before copying its location.".into();
            return;
        }
        self.pending_clipboard = Some(format!("recite://writer{}", self.current_location()));
    }
    pub fn current_location(&self) -> Location {
        let model = self.session.model();
        let mut location = Location {
            project: self
                .session
                .project_root()
                .map(|root| root.to_string_lossy().into_owned()),
            screen: match self.workspaces.screen {
                WorkspaceScreen::Declarations => LinkedScreen::Declarations,
                WorkspaceScreen::Build => LinkedScreen::Build,
                WorkspaceScreen::Rename => LinkedScreen::Rename,
                WorkspaceScreen::External => LinkedScreen::Disk,
                WorkspaceScreen::Localisation => LinkedScreen::Localise,
                WorkspaceScreen::Settings | WorkspaceScreen::Write if self.trial.visible => {
                    LinkedScreen::Preview
                }
                WorkspaceScreen::Settings | WorkspaceScreen::Write
                    if self
                        .rules
                        .as_ref()
                        .is_some_and(|rules| rules.is_open_for(model)) =>
                {
                    LinkedScreen::Rules
                }
                WorkspaceScreen::Settings | WorkspaceScreen::Write => LinkedScreen::Write,
            },
            document: model.document().key().to_string(),
            source: model.view() == &View::Source && self.rules.is_none(),
            beat: model.selected_block().ok().flatten(),
            passage: match model.view() {
                View::Passage(id) => Some(id.clone()),
                _ => self
                    .rules
                    .as_ref()
                    .map(|rules| rules.projection().passage.clone()),
            },
            ..self.location.clone()
        };
        if self.visible_workspace().has_map() {
            location.view = Some(recite_config::WriterView::Map);
            if self.map.workspace == crate::map::Workspace::Map {
                location.beat = self.map_selected().map(str::to_owned);
                location.passage = None;
                location.source = false;
            }
        } else {
            location.view = Some(recite_config::WriterView::Script);
        }
        self.localisation.describe_location(
            self.session.project_root(),
            self.workspaces.screen == WorkspaceScreen::Localisation,
            &mut location,
        );
        location
    }
}
