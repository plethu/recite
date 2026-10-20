//! Project manifest review retains its opening baseline across explicit IDE edits.
mod preparing;
mod publication;
mod reloading;
use super::{FileError, ProjectFiles};
pub use publication::{SettingsCommit, SettingsPublication, SettingsWritten};
use recite_config::ProjectSettings;
use recite_writer_model::Workbench;
pub use reloading::SettingsAccepted;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct Identity(AtomicBool);
#[derive(Clone)]
pub(super) struct ConfigurationChange(Arc<Identity>);
impl ConfigurationChange {
    pub(super) fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Identity {
    fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
enum Activity {
    Idle,
    Preparing(Arc<Identity>),
    Reloading(Arc<Identity>),
    Publishing(Arc<Identity>),
}
struct Snapshot {
    owner: Baseline,
    schema: Option<String>,
    artifact: Option<PathBuf>,
    review: Option<Arc<Review>>,
}
#[derive(Clone)]
enum Baseline {
    Confirmed(Arc<ProjectSettings>),
    Unconfirmed(Arc<ProjectSettings>),
}
impl Baseline {
    fn last_known(&self) -> &Arc<ProjectSettings> {
        match self {
            Self::Confirmed(owner) | Self::Unconfirmed(owner) => owner,
        }
    }
}
struct Review {
    path: PathBuf,
    source: Arc<str>,
    schema: Option<String>,
    documents: BTreeSet<PathBuf>,
}
pub struct Session {
    root: PathBuf,
    snapshot: Arc<Snapshot>,
    activity: Activity,
}
pub enum SettingsAction {
    ExportDraft,
    ImportDraft(PathBuf),
    Reload,
}
#[derive(Clone, Copy)]
pub enum SettingsOperation {
    Export,
    Import,
    Reload,
}
impl SettingsAction {
    fn operation(&self) -> SettingsOperation {
        match self {
            Self::ExportDraft => SettingsOperation::Export,
            Self::ImportDraft(_) => SettingsOperation::Import,
            Self::Reload => SettingsOperation::Reload,
        }
    }
}
pub struct SettingsWork {
    identity: Arc<Identity>,
    root: PathBuf,
    expected: Arc<Snapshot>,
    action: SettingsAction,
    documents: Vec<PathBuf>,
}
pub struct SettingsReceipt {
    identity: Arc<Identity>,
    expected: Arc<Snapshot>,
    result: Option<Result<Arc<Snapshot>, FileError>>,
    retired: Option<Arc<Snapshot>>,
    artifact: Option<PathBuf>,
    retired_declarations: Option<crate::declarations::Session>,
    observation: BaselineObservation,
    operation: SettingsOperation,
}
#[derive(Clone, Copy)]
enum BaselineObservation {
    None,
    Attempted,
}
impl Session {
    pub fn open(root: &Path) -> Result<Self, FileError> {
        let root = root.canonicalize()?;
        let owner = ProjectSettings::open(&root).map_err(settings_error)?;
        if owner.path().parent() != Some(root.as_path()) {
            return Err(FileError::ProjectRootChanged);
        }
        let loaded = recite_core::project::ProjectManifest::load_str_with_spans(
            "recite.project.toml",
            owner.source(),
        );
        let parsed = loaded
            .source
            .ok_or(FileError::Validation(loaded.diagnostics))?;
        Ok(Self {
            root,
            snapshot: Arc::new(Snapshot {
                schema: parsed.manifest().project.schema.clone(),
                owner: Baseline::Confirmed(Arc::new(owner)),
                artifact: None,
                review: None,
            }),
            activity: Activity::Idle,
        })
    }
    pub fn busy(&self) -> bool {
        !matches!(self.activity, Activity::Idle)
    }
    pub fn cancel(&self) {
        if let Activity::Preparing(identity)
        | Activity::Reloading(identity)
        | Activity::Publishing(identity) = &self.activity
        {
            identity.0.store(true, Ordering::Release);
        }
    }
    pub fn path(&self) -> &Path {
        self.snapshot.owner.last_known().path()
    }
    pub fn baseline(&self) -> &str {
        self.snapshot.owner.last_known().source()
    }
    pub fn requires_reload(&self) -> bool {
        matches!(self.snapshot.owner, Baseline::Unconfirmed(_))
    }
    pub fn artifact(&self) -> Option<&Path> {
        self.snapshot.artifact.as_deref()
    }
    pub fn reviewed_path(&self) -> Option<&Path> {
        self.snapshot
            .review
            .as_ref()
            .map(|review| review.path.as_path())
    }
    pub fn reviewed_source(&self) -> Option<&str> {
        self.snapshot
            .review
            .as_ref()
            .map(|review| review.source.as_ref())
    }
    fn prepare(
        &mut self,
        action: SettingsAction,
        documents: Vec<PathBuf>,
    ) -> Result<SettingsWork, FileError> {
        if self.busy() {
            return Err(FileError::SavePending);
        }
        let identity = Arc::new(Identity(AtomicBool::new(false)));
        self.activity = if matches!(action, SettingsAction::Reload) {
            Activity::Reloading(identity.clone())
        } else {
            Activity::Preparing(identity.clone())
        };
        Ok(SettingsWork {
            identity,
            root: self.root.clone(),
            expected: self.snapshot.clone(),
            action,
            documents,
        })
    }
    fn accept(&mut self, receipt: &mut SettingsReceipt) -> Result<(), FileError> {
        let (Activity::Preparing(identity) | Activity::Reloading(identity)) = &self.activity else {
            return Err(FileError::SettingsChanged);
        };
        if !Arc::ptr_eq(identity, &receipt.identity) {
            return Err(FileError::SettingsChanged);
        }
        let observed_reload = matches!(&self.activity, Activity::Reloading(_))
            && matches!(receipt.observation, BaselineObservation::Attempted);
        self.activity = Activity::Idle;
        if receipt.identity.cancelled() && !observed_reload {
            return Err(FileError::SettingsCancelled);
        }
        if !Arc::ptr_eq(&self.snapshot, &receipt.expected) {
            return Err(FileError::SettingsChanged);
        }
        let next = receipt.result.take().ok_or(FileError::SettingsChanged)??;
        receipt.retired = Some(std::mem::replace(&mut self.snapshot, next));
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.cancel();
    }
}
impl ProjectFiles {
    pub fn configuration_refresh_required(&self) -> bool {
        self.configuration_change.is_some()
    }
    pub fn require_navigation_ready(&self) -> Result<(), FileError> {
        self.require_project_edit_ready()?;
        // Freya's legacy editor worker also sets `saving` while executing navigation.
        // The settings publication token is the authority for this membership exclusion.
        if self.settings.as_ref().is_some_and(|session| {
            matches!(
                &session.activity,
                Activity::Publishing(_) | Activity::Reloading(_)
            )
        }) {
            return Err(FileError::SavePending);
        }
        if self.configuration_refresh_required() {
            return Err(FileError::SettingsRefreshRequired);
        }
        Ok(())
    }
    pub fn prepare_settings(&mut self, action: SettingsAction) -> Result<SettingsWork, FileError> {
        self.require_idle_save()?;
        let reloading = matches!(action, SettingsAction::Reload);
        if reloading && self.manifest.dirty() {
            return Err(FileError::ProjectEditPending);
        }
        if reloading
            && self
                .declarations
                .as_ref()
                .is_some_and(|session| session.busy() || session.dirty())
        {
            return Err(FileError::SchemaSessionActive);
        }
        let documents = self.session_paths().cloned().collect();
        let work = self
            .settings
            .as_mut()
            .ok_or(FileError::Selection)?
            .prepare(action, documents)?;
        if reloading {
            self.set_saving(true);
        }
        Ok(work)
    }
}
fn settings_error(error: recite_config::ProjectSettingsError) -> FileError {
    match error {
        recite_config::ProjectSettingsError::Conflict => FileError::SettingsConflict,
        recite_config::ProjectSettingsError::Validation(diagnostics) => {
            FileError::Validation(diagnostics)
        }
        other => FileError::ProjectSettings(other),
    }
}

#[cfg(test)]
mod tests;
