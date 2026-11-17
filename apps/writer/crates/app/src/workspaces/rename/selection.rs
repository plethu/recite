//! Rename beat from Commands: a picker of the open scene's beats chooses the beat, and its name
//! then becomes a field in place, as F2 makes the selected beat's.
use super::inline::{Place, Target};
use crate::map::Workspace;
use crate::navigation::Target as Navigation;
pub use crate::{Phase, Writer, choices::Request, workspaces::Screen};
use recite_writer_model::AnalysisStamp;
use recite_writer_session::choices::{PickerOption, display_name};
use recite_writer_session::location::{Location, Screen as LinkedScreen};

pub struct BeatChoice {
    window: crate::WindowKey,
    source: AnalysisStamp,
    pub(crate) choices: Vec<PickerOption>,
}
impl BeatChoice {
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.source.is_current(writer.session.model().document())
            && writer.phase == Phase::Writing
            && !writer.project_action_blocked()
            && writer.inline_rename().is_none()
    }
    pub(crate) fn apply(self, writer: &mut Writer, beat: String) {
        if !self.is_current(writer) || !self.choices.iter().any(|choice| choice.value == beat) {
            writer.status = "The scene changed. Choose Rename beat again to continue.".into();
            return;
        }
        writer.rename_chosen_beat(beat);
    }
}
impl Writer {
    /// Rename beat from Commands, or F2 with no beat to rename in place: choose the beat first.
    pub fn choose_beat_to_rename(&mut self) {
        if self.phase != Phase::Writing
            || self.project_action_blocked()
            || self.session.project().is_none()
            || self.inline_rename().is_some()
        {
            return;
        }
        let choices = self
            .session
            .model()
            .document()
            .sections()
            .into_iter()
            .map(|value| PickerOption {
                title: display_name(&value),
                detail: value.clone(),
                value,
                annotation: String::new(),
            })
            .collect();
        self.pending_choice = Some(Request::Rename(BeatChoice {
            window: self.window,
            source: self.session.model().document().analysis_stamp(),
            choices,
        }));
    }
    /// The chosen beat's name becomes a field: its card where the map shows it, its heading
    /// otherwise, once Script has selected it.
    fn rename_chosen_beat(&mut self, beat: String) {
        if self.workspaces.screen != Screen::Write {
            self.return_to_writing();
        }
        if self.visible_workspace() == Workspace::Map {
            self.rename_beat_in(Target {
                beat,
                place: Place::Card,
            });
            return;
        }
        if self
            .session
            .model()
            .selected_block()
            .ok()
            .flatten()
            .as_ref()
            == Some(&beat)
        {
            self.rename_beat_in(Target {
                beat,
                place: Place::Heading,
            });
            return;
        }
        let location = Location {
            screen: LinkedScreen::Write,
            beat: Some(beat.clone()),
            passage: None,
            source: false,
            page: 0,
            ..self.current_location()
        };
        self.navigate(Navigation::Selection(location));
        if self.navigation.busy() {
            self.workspaces.rename.chosen = Some(beat);
        }
    }
    /// Once navigation has selected the beat chosen to rename, its heading becomes the field.
    pub(crate) fn resume_chosen_rename(&mut self) {
        if self.navigation.busy() {
            return;
        }
        let Some(beat) = self.workspaces.rename.chosen.take() else {
            return;
        };
        if self
            .session
            .model()
            .selected_block()
            .ok()
            .flatten()
            .as_ref()
            == Some(&beat)
        {
            self.rename_beat_in(Target {
                beat,
                place: Place::Heading,
            });
        }
    }
}

#[cfg(test)]
mod tests;
