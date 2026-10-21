//! SaveSequence owns successful finishing; admission/transport failures still need one refresh.
use super::*;

impl Writer {
    pub(super) fn linked_editor_readiness(&self) -> Result<(), String> {
        match &self.work.after {
            AfterSave::OpenLinkedEditor(link) => link.check(self),
            _ => Ok(()),
        }
    }
    pub(super) fn finish_save_work(&mut self, result: Result<(), FileError>) {
        let readiness = self.linked_editor_readiness();
        let result = match self.session.parts_mut() {
            Some((files, model)) => files.finish_save(model, result),
            None => Err(FileError::ProjectRequired),
        };
        self.complete_save_work(result, readiness);
    }
    pub(super) fn complete_save_work(
        &mut self,
        result: Result<(), FileError>,
        linked_editor_readiness: Result<(), String>,
    ) {
        if let Some((files, model)) = self.session.parts_mut() {
            files.set_saving(false);
            let _ = files.queue_checkpoint(model);
        }
        self.reading.prepare(self.session.model());
        self.status = match &result {
            Ok(()) if self.session.dirty() || self.localisation.dirty() => {
                "Saved; newer edits remain open.".into()
            }
            Ok(()) => "Saved".into(),
            Err(error) => feedback::file(error),
        };
        match std::mem::take(&mut self.work.after) {
            AfterSave::Close
                if result.is_ok() && !self.session.dirty() && !self.localisation.dirty() =>
            {
                self.close_with_recovery()
            }
            AfterSave::Close => self.phase = Phase::ConfirmClose,
            AfterSave::OpenEditor(request) if result.is_ok() => self.launch_editor(request),
            AfterSave::OpenLinkedEditor(link) => {
                // Readiness was checked immediately before the sequence's synchronous
                // terminal refresh. Only that owned save transition intervenes.
                let result = result.map_err(|error| feedback::file(&error));
                link.finish(self, result.and(linked_editor_readiness));
            }
            AfterSave::Build(asset) if result.is_ok() => self.build_selected(asset),
            AfterSave::Declarations if result.is_ok() => self.refresh_declaration_rows(),
            AfterSave::Declarations => {
                if let Err(error) = &result {
                    self.schema_feedback(error.to_string());
                }
            }
            AfterSave::Stay | AfterSave::OpenEditor(_) | AfterSave::Build(_) => {}
        }
        if self.phase == Phase::Writing {
            self.refresh_rules();
            self.invalidate_map();
            self.refresh_map_if_needed();
        }
    }
}
