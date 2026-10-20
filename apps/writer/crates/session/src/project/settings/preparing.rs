use super::*;
use crate::{read_regular, sync_directory};
use std::io::Write;
impl SettingsWork {
    pub fn execute(self) -> SettingsReceipt {
        let mut artifact = None;
        let mut observation = BaselineObservation::None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.prepare(&mut artifact, &mut observation)
        }))
        .unwrap_or_else(|_| {
            Err(std::io::Error::other("Project settings preparation stopped unexpectedly.").into())
        });
        SettingsReceipt {
            identity: self.identity,
            expected: self.expected,
            result: Some(result),
            retired: None,
            artifact,
            retired_declarations: None,
            observation,
            operation: self.action.operation(),
        }
    }
    pub fn not_started(self, error: String) -> SettingsReceipt {
        SettingsReceipt {
            identity: self.identity,
            expected: self.expected,
            result: Some(Err(std::io::Error::other(error).into())),
            retired: None,
            artifact: None,
            retired_declarations: None,
            observation: BaselineObservation::None,
            operation: self.action.operation(),
        }
    }
    fn prepare(
        &self,
        exported: &mut Option<PathBuf>,
        observation: &mut BaselineObservation,
    ) -> Result<Arc<Snapshot>, FileError> {
        if self.identity.cancelled() {
            return Err(FileError::SettingsCancelled);
        }
        match &self.action {
            SettingsAction::Reload => {
                *observation = BaselineObservation::Attempted;
                let session = Session::open(&self.root)?;
                session
                    .snapshot
                    .owner
                    .last_known()
                    .validate_preserving_documents(session.baseline(), &self.documents)
                    .map_err(settings_error)?;
                Ok(session.snapshot.clone())
            }
            SettingsAction::ExportDraft => {
                let source = self
                    .expected
                    .review
                    .as_ref()
                    .map_or(self.expected.owner.last_known().source(), |review| {
                        review.source.as_ref()
                    });
                let mut artifact = tempfile::Builder::new()
                    .prefix(".recite-project-settings-")
                    .suffix(".toml")
                    .tempfile_in(&self.root)?;
                artifact.write_all(source.as_bytes())?;
                artifact.as_file().sync_all()?;
                let (_, path) = artifact.keep().map_err(|error| error.error)?;
                *exported = Some(path.clone());
                sync_directory(&self.root)?;
                Ok(Arc::new(Snapshot {
                    owner: self.expected.owner.clone(),
                    schema: self.expected.schema.clone(),
                    artifact: Some(path),
                    review: self.expected.review.clone(),
                }))
            }
            SettingsAction::ImportDraft(path) => {
                let path = self.root.join(path).canonicalize()?;
                if path == self.expected.owner.last_known().path() {
                    return Err(FileError::SettingsSource);
                }
                let source = read_regular(&path)?;
                let report = self
                    .expected
                    .owner
                    .last_known()
                    .validate_preserving_documents(&source, &self.documents)
                    .map_err(settings_error)?;
                let review = Review {
                    path,
                    source: source.into(),
                    schema: report.manifest().source().manifest().project.schema.clone(),
                    documents: report
                        .documents()
                        .iter()
                        .map(|document| document.path().to_owned())
                        .collect(),
                };
                Ok(Arc::new(Snapshot {
                    owner: self.expected.owner.clone(),
                    schema: self.expected.schema.clone(),
                    artifact: self.expected.artifact.clone(),
                    review: Some(Arc::new(review)),
                }))
            }
        }
    }
}
impl SettingsReceipt {
    pub fn operation(&self) -> SettingsOperation {
        self.operation
    }
    pub fn preview(&self) -> Option<&str> {
        let snapshot = self.result.as_ref()?.as_ref().ok()?;
        Some(
            snapshot
                .review
                .as_ref()
                .map_or(snapshot.owner.last_known().source(), |review| {
                    review.source.as_ref()
                }),
        )
    }
    pub fn artifact(&self) -> Option<&Path> {
        self.artifact.as_deref()
    }
}
