//! Publish receipts against live buffers before retiring displaced worker owners.
use super::*;
impl SaveSequence {
    pub fn accept(&mut self, files: &mut ProjectFiles, current: &mut Workbench, receipt: Receipt) {
        let pending = self.pending.take();
        let result = match (pending, receipt) {
            (Some(Pending::Edit(path)), Receipt::Edit(result)) => {
                self.accept_edit(files, path, result)
            }
            (Some(Pending::Write(path)), Receipt::Written(mut write, result)) => {
                let publication = result.and_then(|()| files.publish_write(current, &mut write));
                let next = match self.file_result(path.clone(), publication) {
                    Ok(ticket) => Some((Work::Recovery(ticket), Pending::Recovery(path))),
                    Err(error) => {
                        self.record(error);
                        None
                    }
                };
                self.retire(Work::RetireWrite(write), next);
                Ok(())
            }
            (Some(Pending::Recovery(path)), Receipt::Recovered(result)) => {
                let parked = files.finish_saved_tab(&path);
                self.file_result(path, result.and(parked))
            }
            (Some(Pending::Manifest), Receipt::Manifest(outcome)) => {
                files.accept_manifest_save(outcome)
            }
            (Some(Pending::DeclarationSource), Receipt::DeclarationSource(request, result)) => {
                self.accept_source(files, request, result);
                Ok(())
            }
            (
                Some(Pending::DeclarationRecovery),
                Receipt::DeclarationRecovered(request, result),
            ) => {
                if let Err(error) = result {
                    self.record(error);
                    self.retire(Work::RetireDeclaration(request), None);
                } else {
                    self.queued =
                        Some((Work::DeclarationOutput(request), Pending::DeclarationOutput));
                }
                Ok(())
            }
            (Some(Pending::DeclarationOutput), Receipt::DeclarationOutput(request, result)) => {
                self.accept_output(files, current, request, result)
            }
            (Some(Pending::Retire), Receipt::Retired) => {
                self.queued = self.after_retire.take();
                Ok(())
            }
            _ => Err(FileError::Selection),
        };
        if let Err(error) = result {
            self.record(error);
        }
    }
    fn accept_edit(
        &mut self,
        files: &mut ProjectFiles,
        path: PathBuf,
        result: Result<Box<recite_writer_model::PreparedEdit>, recite_writer_model::WorkbenchError>,
    ) -> Result<(), FileError> {
        let mut prepared = self.file_result(path.clone(), result.map_err(FileError::from))?;
        let accepted = self.file_result(
            path.clone(),
            files.accept_retained_edit(&path, &mut prepared),
        );
        if accepted.is_ok() {
            self.steps.push_front(Step::Retained {
                path,
                drafts: Drafts::Retain,
            });
        }
        self.retire(Work::RetireEdit(prepared), None);
        accepted
    }
    fn accept_source(
        &mut self,
        files: &mut ProjectFiles,
        request: Box<crate::declarations::DeclarationSave>,
        result: Result<(), FileError>,
    ) {
        let publication = result.and_then(|()| {
            files
                .declarations
                .as_mut()
                .ok_or(FileError::Selection)?
                .accept_saved_source(&request)
        });
        match publication {
            Ok(ticket) => {
                self.queued = Some((
                    Work::DeclarationRecovery(request, ticket),
                    Pending::DeclarationRecovery,
                ));
            }
            Err(error) => {
                self.record(error);
                self.retire(Work::RetireDeclaration(request), None);
            }
        }
    }
    fn accept_output(
        &mut self,
        files: &mut ProjectFiles,
        current: &mut Workbench,
        request: Box<crate::declarations::DeclarationSave>,
        result: Result<(), FileError>,
    ) -> Result<(), FileError> {
        if let Err(error) = result {
            self.record(error);
            self.retire(Work::RetireDeclaration(request), None);
            return Ok(());
        }
        files
            .declarations
            .as_mut()
            .ok_or(FileError::Selection)?
            .accept_saved_output(*request);
        // Refresh validates the published schema while preserving newer source drafts.
        files.set_saving(false);
        let result = files.refresh(current);
        files.set_saving(true);
        result
    }
}
