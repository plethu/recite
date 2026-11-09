//! Cold loading already runs on a worker; apply its initial location before installation.
use super::*;

impl Writer {
    pub fn follow_location(
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
                Session::Working { .. } | Session::Recovering { .. } => {
                    return Err(FileError::ProjectActionPending.to_string());
                }
            }
        }
        if self.source_repair_pending() {
            return Err("Repair and import the recovered source draft before following this link. Your draft remains open.".into());
        }
        let selection = location::selection_location(location);
        let mut rules = None;
        self.session
            .edit(|model| {
                if model.has_draft() {
                    model.apply()?;
                }
                rules = recite_writer_session::location::select(model, &selection)?;
                Ok(())
            })
            .map_err(|error| error.to_string())?;
        self.install_rules(&mut rules);
        self.location = location.clone();
        self.trial.visible = location.screen == recite_writer_session::location::Screen::Preview;
        self.reading.prepare(self.session.model());
        self.prepare_initial_location_destination(location)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
