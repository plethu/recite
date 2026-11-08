//! The scene picker chooses a proposal value without applying or navigating the manuscript.
use crate::{Phase, Writer, choices::Request, workspaces::Screen};
use recite_writer_model::AnalysisStamp;
use recite_writer_session::choices::{PickerOption, display_name};

pub struct BeatChoice {
    window: crate::WindowKey,
    source: AnalysisStamp,
    block: String,
    name: String,
    pub choices: Vec<PickerOption>,
}
impl BeatChoice {
    pub fn is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.source.is_current(writer.session.model().document())
            && writer.phase == Phase::Writing
            && writer.workspaces.screen == Screen::Rename
            && !writer.project_action_blocked()
            && self.block == writer.workspaces.rename.block
            && self.name == writer.workspaces.rename.name
    }
    pub fn apply(self, writer: &mut Writer, beat: String) {
        if !self.is_current(writer) || !self.choices.iter().any(|choice| choice.value == beat) {
            writer.status =
                "The rename proposal changed. Reopen the beat choice to continue.".into();
            return;
        }
        if let Err(error) = writer
            .session
            .model()
            .document()
            .validate_script_block(&beat)
        {
            writer.status = error.to_string();
            return;
        }
        if writer.workspaces.rename.block != beat {
            writer.change_rename_block(beat);
        }
    }
}
impl Writer {
    pub fn choose_rename_beat(&mut self) {
        if self.workspaces.screen != Screen::Rename
            || self.project_action_blocked()
            || self.session.project().is_none()
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
            block: self.workspaces.rename.block.clone(),
            name: self.workspaces.rename.name.clone(),
            choices,
        }));
    }
}

#[cfg(test)]
mod tests;
