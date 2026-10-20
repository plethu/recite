//! Checked declaration actions retain the live recovery lease and newer source typing.
mod accepting;
mod inspection;
pub use inspection::SourceStatus;
pub(super) use inspection::inspect_source;
mod finishing;
mod metadata;
use finishing::FinishPayload;
pub use finishing::{DeclarationFinish, DeclarationFinished};
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

pub enum DeclarationAction {
    InspectSource,
    Bind(PathBuf),
    ReloadSource,
    Discard,
    ExportDraft,
    ImportDraft(PathBuf),
    ReloadGenerated,
    ReloadRegistration,
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
enum Inputs {
    Source(Option<SourceStamp>),
    Output,
}
impl Inputs {
    fn source(&self) -> Option<&SourceStamp> {
        match self {
            Self::Source(source) => source.as_ref(),
            Self::Output => None,
        }
    }
    fn matches(&self, source: Option<&Source>) -> bool {
        match (self, source) {
            (Self::Output, _) | (Self::Source(None), None) => true,
            (Self::Source(Some(stamp)), Some(source)) => stamp.matches(source),
            _ => false,
        }
    }
}
pub struct DeclarationWork {
    token: ActivityToken,
    action: DeclarationAction,
    root: PathBuf,
    output: PathBuf,
    schema_output: Arc<SchemaOutput>,
    inputs: Inputs,
}
struct Replacement {
    baseline: String,
    draft: String,
    output: Option<Arc<SchemaOutput>>,
}
enum PreparedDeclaration {
    Bound(Box<Source>),
    Replaced(Replacement),
    Unchanged,
    Output(Arc<SchemaOutput>),
    Registration(Result<Option<recite_config::ProducerRegistration>, String>),
}
pub struct DeclarationReceipt {
    token: ActivityToken,
    schema_output: Arc<SchemaOutput>,
    inputs: Inputs,
    artifact: Option<SourceArtifact>,
    result: Option<Result<PreparedDeclaration, FileError>>,
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
    pub fn prepare_declaration(
        &mut self,
        action: DeclarationAction,
    ) -> Result<DeclarationWork, FileError> {
        self.require_idle_save()?;
        if self.busy() || (matches!(&action, DeclarationAction::Bind(_)) && self.dirty()) {
            return Err(FileError::UnsavedDocument);
        }
        if matches!(
            &action,
            DeclarationAction::ReloadSource
                | DeclarationAction::ExportDraft
                | DeclarationAction::ImportDraft(_)
        ) && self.source.is_none()
        {
            return Err(FileError::SchemaOwnership);
        }
        let token = ActivityToken::new();
        self.activity = Activity::PreparingDeclaration(token.clone());
        let inputs = match &action {
            DeclarationAction::ReloadGenerated | DeclarationAction::ReloadRegistration => {
                Inputs::Output
            }
            _ => Inputs::Source(self.source.as_ref().map(SourceStamp::capture)),
        };
        Ok(DeclarationWork {
            token,
            action,
            root: self.root.clone(),
            output: self.output.clone(),
            schema_output: self.schema_output.clone(),
            inputs,
        })
    }
    fn declaration_blocking(
        &mut self,
        action: DeclarationAction,
    ) -> Result<Option<PathBuf>, FileError> {
        let mut receipt = self.prepare_declaration(action)?.execute();
        let work = self.accept_declaration(&mut receipt)?;
        self.accept_declaration_finish(&mut work.execute())?;
        Ok(receipt.artifact.and_then(|artifact| match artifact {
            SourceArtifact::Archived(path) => Some(path),
            SourceArtifact::Editable(_) => None,
        }))
    }
    pub fn bind(&mut self, path: &Path) -> Result<(), FileError> {
        self.declaration_blocking(DeclarationAction::Bind(path.to_owned()))
            .map(drop)
    }
    /// The captured draft is archived before preparing an explicit disk rebase.
    pub fn reload_source(&mut self) -> Result<PathBuf, FileError> {
        self.declaration_blocking(DeclarationAction::ReloadSource)?
            .ok_or(FileError::SchemaOwnership)
    }
    pub fn discard(&mut self) -> Result<(), FileError> {
        self.declaration_blocking(DeclarationAction::Discard)
            .map(drop)
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
impl DeclarationWork {
    pub fn execute(self) -> DeclarationReceipt {
        let mut artifact = None;
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.prepare(&mut artifact)))
                .unwrap_or_else(|_| {
                    Err(std::io::Error::other(
                        "Declaration source preparation stopped unexpectedly.",
                    )
                    .into())
                });
        DeclarationReceipt {
            token: self.token,
            schema_output: self.schema_output,
            inputs: self.inputs,
            artifact,
            result: Some(result),
        }
    }
    pub fn not_started(self, error: String) -> DeclarationReceipt {
        DeclarationReceipt {
            token: self.token,
            schema_output: self.schema_output,
            inputs: self.inputs,
            artifact: None,
            result: Some(Err(std::io::Error::other(error).into())),
        }
    }
}

#[cfg(test)]
mod tests;
