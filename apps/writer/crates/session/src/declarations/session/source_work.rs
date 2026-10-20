//! Source preparation keeps the live recovery lease and all newer typing in the Session.
mod accepting;
mod inspection;
pub use inspection::SourceStatus;
pub(super) use inspection::inspect_source;
mod preparing;
use super::{
    DeclarationRevision, FileError, ProjectSchema, SchemaOutput, SchemaOutputStatus, Session,
    Source,
    activity::{Activity, ActivityToken},
};
use crate::{declarations::recovery::DraftSnapshot, recovery::worker::Durability};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub enum SourceAction {
    Inspect,
    Bind(PathBuf),
    Reload,
    Discard,
    ExportDraft,
    ImportDraft(PathBuf),
}
enum SourceArtifact {
    Archived(PathBuf),
    Editable(PathBuf),
}
struct SourceStamp {
    path: PathBuf,
    revision: DeclarationRevision,
    baseline: String,
    draft: String,
}
pub struct SourceWork {
    token: ActivityToken,
    action: SourceAction,
    root: PathBuf,
    output: PathBuf,
    schema_output: Arc<SchemaOutput>,
    source: Option<SourceStamp>,
}
struct Replacement {
    baseline: String,
    draft: String,
    output: Option<Arc<SchemaOutput>>,
}
enum PreparedSource {
    Bound(Box<Source>),
    Replaced(Replacement),
    Unchanged,
}
pub struct SourceReceipt {
    token: ActivityToken,
    schema_output: Arc<SchemaOutput>,
    source: Option<SourceStamp>,
    artifact: Option<SourceArtifact>,
    result: Option<Result<PreparedSource, FileError>>,
}
/// Adoption has queued recovery; the activity stays busy until its fixed fence is harvested.
pub struct SourceRecovery {
    token: ActivityToken,
    ticket: Option<Durability<DraftSnapshot>>,
    retired_source: Option<Source>,
    retired_state: Option<Replacement>,
}
pub struct SourceRecovered {
    token: ActivityToken,
    result: Option<Result<(), FileError>>,
}
impl SourceStamp {
    fn capture(source: &Source) -> Self {
        Self {
            path: source.path.clone(),
            revision: source.revision,
            baseline: source.baseline.clone(),
            draft: source.draft.clone(),
        }
    }
    fn matches(&self, source: &Source) -> bool {
        self.path == source.path
            && self.revision == source.revision
            && self.baseline == source.baseline
            && self.draft == source.draft
    }
}
impl Session {
    pub fn prepare_source(&mut self, action: SourceAction) -> Result<SourceWork, FileError> {
        self.require_idle_save()?;
        if self.busy() || (matches!(&action, SourceAction::Bind(_)) && self.dirty()) {
            return Err(FileError::UnsavedDocument);
        }
        if matches!(
            &action,
            SourceAction::Reload | SourceAction::ExportDraft | SourceAction::ImportDraft(_)
        ) && self.source.is_none()
        {
            return Err(FileError::SchemaOwnership);
        }
        let token = ActivityToken::new();
        self.activity = Activity::ChangingSource(token.clone());
        Ok(SourceWork {
            token,
            action,
            root: self.root.clone(),
            output: self.output.clone(),
            schema_output: self.schema_output.clone(),
            source: self.source.as_ref().map(SourceStamp::capture),
        })
    }
    fn source_blocking(&mut self, action: SourceAction) -> Result<Option<PathBuf>, FileError> {
        let mut receipt = self.prepare_source(action)?.execute();
        let work = self.accept_source(&mut receipt)?;
        self.accept_source_recovery(&mut work.execute())?;
        Ok(receipt.artifact.and_then(|artifact| match artifact {
            SourceArtifact::Archived(path) => Some(path),
            SourceArtifact::Editable(_) => None,
        }))
    }
    pub fn bind(&mut self, path: &Path) -> Result<(), FileError> {
        self.source_blocking(SourceAction::Bind(path.to_owned()))
            .map(drop)
    }
    /// The captured draft is archived before preparing an explicit disk rebase.
    pub fn reload_source(&mut self) -> Result<PathBuf, FileError> {
        self.source_blocking(SourceAction::Reload)?
            .ok_or(FileError::SchemaOwnership)
    }
    pub fn discard(&mut self) -> Result<(), FileError> {
        self.source_blocking(SourceAction::Discard).map(drop)
    }
    pub fn generation_active(&self) -> bool {
        matches!(
            &self.activity,
            Activity::StartingGeneration(_)
                | Activity::Generating(_)
                | Activity::PublishingGeneration(_)
        )
    }
}
impl SourceWork {
    pub fn execute(self) -> SourceReceipt {
        let mut artifact = None;
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.prepare(&mut artifact)))
                .unwrap_or_else(|_| {
                    Err(std::io::Error::other(
                        "Declaration source preparation stopped unexpectedly.",
                    )
                    .into())
                });
        SourceReceipt {
            token: self.token,
            schema_output: self.schema_output,
            source: self.source,
            artifact,
            result: Some(result),
        }
    }
    pub fn not_started(self, error: String) -> SourceReceipt {
        SourceReceipt {
            token: self.token,
            schema_output: self.schema_output,
            source: self.source,
            artifact: None,
            result: Some(Err(std::io::Error::other(error).into())),
        }
    }
}
impl SourceRecovery {
    pub fn execute(self) -> SourceRecovered {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let result = self.ticket.map_or(Ok(()), Durability::wait);
            drop(self.retired_source);
            drop(self.retired_state);
            result
        }))
        .unwrap_or_else(|_| {
            Err(std::io::Error::other("Declaration source recovery stopped unexpectedly.").into())
        });
        SourceRecovered {
            token: self.token,
            result: Some(result),
        }
    }
}

#[cfg(test)]
mod tests;
