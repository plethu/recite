use super::*;
impl Writer {
    pub(in crate::localisation) fn source_catalogue_action(&mut self, action: Action) {
        if self.project_action_blocked()
            || self.trial.busy()
            || self.localisation.slot.read_only()
            || self.localisation.saving()
        {
            return;
        }
        let project = match self.session.checkout() {
            Ok(project) => project,
            Err(error) => {
                self.catalogue_feedback(error.to_string());
                return;
            }
        };
        let selected = self.localisation.mounted_entries();
        match self.localisation.slot.checkout(selected) {
            Ok(catalogue) => self.submit_catalogue_tool(super::super::Work {
                kind: Kind::Source(Work {
                    project,
                    catalogue,
                    action,
                    query: self.localisation.query.clone(),
                }),
            }),
            Err(error) => {
                let mut project = project;
                let restored = self.session.restore_checkout(&mut project, None);
                self.catalogue_feedback(restored.err().map_or(error, |e| e.to_string()));
            }
        }
    }
    pub(crate) fn accept_catalogue_source(
        &mut self,
        completed: &mut Completed,
    ) -> Result<(), String> {
        let project = self
            .session
            .restore_checkout(&mut completed.project, completed.recovery)
            .map_err(|e| e.to_string());
        let catalogue = self.localisation.slot.restore(&mut completed.catalogue);
        project.and(catalogue)?;
        if self.localisation.slot.identity() != completed.previous.as_ref() {
            completed.retired_entry = self.localisation.entry.take();
            completed.retired_comparison = self.localisation.comparison.take();
            completed.retired_refresh = self.localisation.refresh.take();
            completed.retired_beat = self.localisation.passage.take();
            self.localisation.destination = Destination::Queue;
            self.localisation.files_open = false;
            if let Some(path) = self.localisation.slot.path() {
                self.localisation.path = path.display().to_string();
            }
            self.localisation.queue = Queue::default();
        }
        match &mut completed.result {
            Ok(Outcome::Review(incoming)) => {
                let refresh = incoming
                    .take()
                    .ok_or("Source review was already accepted.")?;
                completed.retired_refresh = self.localisation.refresh.replace(refresh);
                self.localisation.destination = Destination::Refresh;
                self.catalogue_feedback("Source update review ready. Apply rechecks the source and writes only the PO file.".into());
            }
            Ok(Outcome::Queue { queue, message }) => {
                self.localisation.queue = std::mem::take(queue);
                self.localisation.query.page = self.localisation.queue.page;
                self.catalogue_feedback(std::mem::take(message));
            }
            Err(error) => self.catalogue_feedback(error.clone()),
        }
        if completed.result.is_ok() {
            self.remember_catalogue_location();
        }
        Ok(())
    }
}
