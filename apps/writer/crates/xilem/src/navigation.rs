//! Authoring selection and document navigation use the shared product locations.
use super::{Writer, feedback, session::Session};
use recite_writer_model::View;
use recite_writer_session::FileError;
impl Writer {
    pub(super) fn switch(&mut self, path: &std::path::Path) {
        self.trial.cancel();
        self.status = match self
            .session
            .project_mut()
            .ok_or(FileError::ProjectRequired)
            .and_then(|project| project.switch(path))
        {
            Ok(()) => {
                self.beat_page = 0;
                if self.session.model().has_draft()
                    || self
                        .session
                        .project()
                        .is_some_and(|project| project.current().dirty())
                {
                    "Unsaved work restored"
                } else {
                    "Ready"
                }
                .into()
            }
            Err(error) => feedback::file(&error),
        };
        self.reading.prepare(self.session.model());
    }

    pub(super) fn select_example(&mut self, index: usize) {
        self.trial.cancel();
        self.status = match self.session.select_example(index) {
            Ok(()) => "Temporary example opened; drafts and history remain in this window.".into(),
            Err(error) => feedback::file(&error),
        };
        self.beat_page = 0;
        self.reading.prepare(self.session.model());
    }
    pub(super) fn follow_location(
        &mut self,
        location: &recite_writer_session::location::Location,
    ) -> Result<(), String> {
        if self.work.busy() {
            return Err("Wait for the current editor action before opening a link.".into());
        }
        if !location.document.is_empty()
            && location.document != self.session.model().document().key().as_str()
        {
            match &mut self.session {
                Session::Project(project) => {
                    let path = project
                        .files()
                        .path_for_document(&location.document)
                        .ok_or("The linked document is not in this project.")?;
                    project.switch(&path).map_err(|error| error.to_string())?;
                }
                Session::Examples { .. } => {
                    let index = recite_writer_model::WRITER_EXAMPLES
                        .iter()
                        .position(|example| example.name == location.document)
                        .ok_or("The linked example is unavailable.")?;
                    self.session
                        .select_example(index)
                        .map_err(|error| error.to_string())?;
                }
            }
        }
        let mut rules = None;
        self.session
            .edit(|model| {
                if model.has_draft() {
                    model.apply()?;
                }
                rules = recite_writer_session::location::select(model, location)?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.rules = rules;
        self.location = location.clone();
        self.trial.visible = location.screen == recite_writer_session::location::Screen::Preview;
        self.reading.prepare(self.session.model());
        Ok(())
    }
    pub(super) fn select(&mut self, id: String) {
        self.change(|model| model.select(View::Passage(id)));
    }
    pub(super) fn inspect(&mut self, id: String) {
        self.change(|model| model.inspect_block(&id));
    }
    pub(super) fn read_current(&mut self) {
        match self.session.model().selected_block() {
            Ok(Some(id)) => self.inspect(id),
            Ok(None) => self.status = "Choose a beat in the outline.".into(),
            Err(error) => self.status = feedback::workbench(&error),
        }
    }
}
