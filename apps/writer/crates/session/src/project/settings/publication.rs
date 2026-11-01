//! Manifest publication briefly excludes membership changes, then reconciles before refresh.
use super::*;
pub struct SettingsPublication {
    identity: Arc<Identity>,
    expected: Arc<Snapshot>,
    documents: Vec<PathBuf>,
}
pub struct SettingsWritten {
    identity: Arc<Identity>,
    expected: Arc<Snapshot>,
    outcome: Option<WriteOutcome>,
    retired: Option<Arc<Snapshot>>,
    retired_declarations: Option<crate::declarations::Session>,
}
impl ProjectFiles {
    pub fn prepare_settings_publication(
        &mut self,
        current: &Workbench,
    ) -> Result<SettingsPublication, FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        if self.project_edit_pending(current) {
            return Err(FileError::ProjectEditPending);
        }
        if self.builds.busy()
            || self
                .declarations
                .as_ref()
                .is_some_and(|session| session.busy() || session.dirty())
        {
            return Err(FileError::SchemaSessionActive);
        }
        let session = self.settings.as_ref().ok_or(FileError::Selection)?;
        if session.requires_reload() {
            return Err(FileError::SettingsReloadRequired);
        }
        if session.busy() {
            return Err(FileError::SavePending);
        }
        let review = session
            .snapshot
            .review
            .as_ref()
            .ok_or(FileError::SettingsSource)?;
        let documents = self.session_paths().cloned().collect::<Vec<_>>();
        if let Some(path) = documents
            .iter()
            .find(|path| !review.documents.contains(*path))
        {
            return Err(settings_error(
                recite_config::ProjectSettingsError::OpenDocument(path.clone()),
            ));
        }
        let identity = Arc::new(Identity(AtomicBool::new(false)));
        let expected = session.snapshot.clone();
        self.settings.as_mut().ok_or(FileError::Selection)?.activity =
            Activity::Publishing(identity.clone());
        self.set_saving(true);
        Ok(SettingsPublication {
            identity,
            expected,
            documents,
        })
    }
    pub fn accept_settings_publication(
        &mut self,
        receipt: &mut SettingsWritten,
    ) -> Result<SettingsCommit, FileError> {
        let session = self.settings.as_mut().ok_or(FileError::SettingsChanged)?;
        let Activity::Publishing(identity) = &session.activity else {
            return Err(FileError::SettingsChanged);
        };
        if !Arc::ptr_eq(identity, &receipt.identity)
            || !Arc::ptr_eq(&session.snapshot, &receipt.expected)
        {
            return Err(FileError::SettingsChanged);
        }
        let outcome = receipt.outcome.take().ok_or(FileError::SettingsChanged)?;
        session.activity = Activity::Idle;
        self.set_saving(false);
        let (next, commit) = match outcome {
            WriteOutcome::Published { next, warning } => {
                self.manifest
                    .refresh(next.owner.last_known().source().to_owned());
                (
                    next,
                    warning.map_or(
                        SettingsCommit::Published,
                        SettingsCommit::PublishedWithWarning,
                    ),
                )
            }
            WriteOutcome::Unconfirmed { next, error } => (next, SettingsCommit::Unconfirmed(error)),
            WriteOutcome::Unchanged(error) => return Err(error),
        };
        self.configuration_change = Some(ConfigurationChange(receipt.identity.clone()));
        // Obsolete or unconfirmed producer authority retires before any context-refresh attempt.
        if matches!(next.owner, Baseline::Unconfirmed(_))
            || self
                .declarations
                .as_ref()
                .is_some_and(|declarations| !declarations.matches_schema(next.schema.as_deref()))
        {
            receipt.retired_declarations = self.declarations.take();
        }
        let session = self.settings.as_mut().ok_or(FileError::SettingsChanged)?;
        receipt.retired = Some(std::mem::replace(&mut session.snapshot, next));
        Ok(commit)
    }
}
#[derive(Debug)]
pub enum SettingsCommit {
    Published,
    PublishedWithWarning(FileError),
    Unconfirmed(FileError),
}
enum WriteOutcome {
    Published {
        next: Arc<Snapshot>,
        warning: Option<FileError>,
    },
    Unconfirmed {
        next: Arc<Snapshot>,
        error: FileError,
    },
    Unchanged(FileError),
}
impl SettingsPublication {
    pub fn execute(self) -> SettingsWritten {
        self.execute_with(ProjectSettings::save_preserving_documents)
    }
    pub(super) fn execute_with(
        self,
        save: impl FnOnce(
            &mut ProjectSettings,
            &str,
            &[PathBuf],
        ) -> Result<(), recite_config::ProjectSettingsError>,
    ) -> SettingsWritten {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.publish(save)))
            .unwrap_or_else(|_| self.unconfirmed(std::io::Error::other("Project settings publication stopped unexpectedly; reload to confirm its outcome.").into()));
        SettingsWritten {
            identity: self.identity,
            expected: self.expected,
            outcome: Some(outcome),
            retired: None,
            retired_declarations: None,
        }
    }
    pub fn not_started(self, error: String) -> SettingsWritten {
        SettingsWritten {
            identity: self.identity,
            expected: self.expected,
            outcome: Some(WriteOutcome::Unchanged(std::io::Error::other(error).into())),
            retired: None,
            retired_declarations: None,
        }
    }
    fn publish(
        &self,
        save: impl FnOnce(
            &mut ProjectSettings,
            &str,
            &[PathBuf],
        ) -> Result<(), recite_config::ProjectSettingsError>,
    ) -> WriteOutcome {
        if self.identity.cancelled() {
            return WriteOutcome::Unchanged(FileError::SettingsCancelled);
        }
        let Some(review) = self.expected.review.as_ref() else {
            return WriteOutcome::Unchanged(FileError::SettingsSource);
        };
        let mut owner = self.expected.owner.last_known().as_ref().clone();
        match save(&mut owner, &review.source, &self.documents) {
            Ok(()) => self.published(owner, review, None),
            Err(error)
                if matches!(
                    &error,
                    recite_config::ProjectSettingsError::Storage(
                        recite_config::ConfigWriteError::Commit { .. }
                    )
                ) =>
            {
                self.reconcile_commit(error, review)
            }
            Err(error @ recite_config::ProjectSettingsError::Conflict) => {
                self.unconfirmed(settings_error(error))
            }
            Err(error) => WriteOutcome::Unchanged(settings_error(error)),
        }
    }
    fn published(
        &self,
        owner: ProjectSettings,
        review: &Review,
        warning: Option<FileError>,
    ) -> WriteOutcome {
        WriteOutcome::Published {
            next: Arc::new(Snapshot {
                owner: Baseline::Confirmed(Arc::new(owner)),
                schema: review.schema.clone(),
                artifact: self.expected.artifact.clone(),
                review: None,
            }),
            warning,
        }
    }
    fn reconcile_commit(
        &self,
        error: recite_config::ProjectSettingsError,
        review: &Review,
    ) -> WriteOutcome {
        let expected = self.expected.owner.last_known();
        let observed = expected
            .path()
            .parent()
            .and_then(|root| ProjectSettings::open(root).ok());
        match observed {
            Some(owner)
                if owner.path() == expected.path() && owner.source() == review.source.as_ref() =>
            {
                self.published(owner, review, Some(settings_error(error)))
            }
            Some(owner)
                if owner.path() == expected.path() && owner.source() == expected.source() =>
            {
                WriteOutcome::Unchanged(settings_error(error))
            }
            _ => self.unconfirmed(settings_error(error)),
        }
    }
    fn unconfirmed(&self, error: FileError) -> WriteOutcome {
        WriteOutcome::Unconfirmed {
            next: Arc::new(Snapshot {
                owner: Baseline::Unconfirmed(self.expected.owner.last_known().clone()),
                schema: self.expected.schema.clone(),
                artifact: self.expected.artifact.clone(),
                review: self.expected.review.clone(),
            }),
            error,
        }
    }
}
