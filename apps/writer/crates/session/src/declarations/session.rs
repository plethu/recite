//! Explicit standalone source association; generated manifests are never edited.
use crate::project::{FileError, read_regular, save::replace_checked};
use recite_core::schema::{ProjectSchema, SchemaSource};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
mod generation;
mod output;
use output::SchemaOutput;
pub use output::SchemaOutputStatus;
mod saving;
mod source_work;
mod starting;
pub use source_work::{
    SourceAction, SourceReceipt, SourceRecovered, SourceRecovery, SourceStatus, SourceWork,
};
mod activity;
use activity::Activity;
pub use generation::{GenerationReceipt, GenerationWork};
pub use saving::{DeclarationSave, DeclarationSaveRequest};
pub use starting::{GenerationStart, GenerationStarted};

/// Freshness of the mounted declaration source buffer, separate from recovery ordering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeclarationRevision(u64);
impl DeclarationRevision {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

pub struct Session {
    saving: bool,
    pub output: PathBuf,
    schema_output: Arc<SchemaOutput>,
    pub source: Option<Source>,
    pub registration: Result<Option<recite_config::ProducerRegistration>, String>,
    activity: Activity,
    root: PathBuf,
}
pub struct Source {
    pub path: PathBuf,
    pub baseline: String,
    pub recovery: super::recovery::Store,
    pub draft: String,
    pub revision: DeclarationRevision,
}
impl Session {
    pub fn set_saving(&mut self, saving: bool) {
        self.saving = saving;
    }
    fn require_idle_save(&self) -> Result<(), FileError> {
        if self.saving {
            Err(FileError::SavePending)
        } else {
            Ok(())
        }
    }
    pub fn open(root: &Path) -> Result<Self, FileError> {
        let root = root.canonicalize()?;
        let report = recite_config::discover_project(&root)?;
        if report.manifest().project_root() != root {
            return Err(FileError::ProjectRootChanged);
        }
        let relative = report
            .manifest()
            .source()
            .manifest()
            .project
            .schema
            .as_ref()
            .ok_or(FileError::NoSchema)?;
        let output = report.manifest().project_root().join(relative);
        let output_baseline = read_regular(&output)?;
        let loaded = recite_core::schema::load_schema_manifest_str(
            output.to_string_lossy(),
            &output_baseline,
        );
        let schema = loaded
            .schema
            .ok_or(FileError::Validation(loaded.diagnostics))?;
        let registration = Self::registration(&root, &schema);
        Ok(Self {
            saving: false,
            registration,
            activity: Activity::Idle,
            root: root.to_owned(),
            output,
            schema_output: Arc::new(SchemaOutput::Ready(super::producer::Generated {
                schema: schema.into(),
                text: output_baseline,
            })),
            source: None,
        })
    }
    fn registration(
        root: &Path,
        schema: &ProjectSchema,
    ) -> Result<Option<recite_config::ProducerRegistration>, String> {
        Self::read_registration(
            root,
            schema
                .producer_metadata
                .as_ref()
                .and_then(|m| m.producer.as_ref()),
        )
    }
    fn read_registration(
        root: &Path,
        producer: Option<&recite_core::schema::ProducerIdentity>,
    ) -> Result<Option<recite_config::ProducerRegistration>, String> {
        let path = root.join(recite_config::PRODUCER_REGISTRATION_FILE);
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let registration =
            recite_config::ProducerRegistration::parse(&text).map_err(|e| e.to_string())?;
        if producer != Some(registration.producer()) {
            return Err("Registration belongs to a different schema producer.".into());
        }
        Ok(Some(registration))
    }
    pub fn matches_schema(&self, relative: Option<&str>) -> bool {
        relative.is_some_and(|path| self.root.join(path) == self.output)
    }
    pub fn check_project_settings(&self, source: &str) -> Result<(), FileError> {
        let loaded = recite_core::project::ProjectManifest::load_str_with_spans(
            "recite.project.toml",
            source,
        );
        let source = loaded
            .source
            .ok_or(FileError::Validation(loaded.diagnostics))?;
        if !self.matches_schema(source.manifest().project.schema.as_deref())
            && (self.dirty() || self.busy())
        {
            return Err(FileError::SchemaSessionActive);
        }
        Ok(())
    }
    pub fn reload_generated(&mut self) -> Result<(), FileError> {
        self.require_idle_save()?;
        if self.busy() {
            return Err(FileError::UnsavedDocument);
        }
        let text = read_regular(&self.output)?;
        let loaded =
            recite_core::schema::load_schema_manifest_str(self.output.to_string_lossy(), &text);
        let schema = loaded
            .schema
            .ok_or(FileError::Validation(loaded.diagnostics))?;
        self.check_owner(&schema)?;
        self.schema_output = Arc::new(SchemaOutput::Ready(super::producer::Generated {
            schema: schema.into(),
            text,
        }));
        Ok(())
    }
    pub fn reload_registration(&mut self) {
        if !self.busy() {
            self.registration = Self::registration(&self.root, self.last_valid_schema());
        }
    }
    pub fn cancel(&self) {
        self.activity.cancel();
    }
    pub fn busy(&self) -> bool {
        !matches!(self.activity, Activity::Idle)
    }
    pub fn dirty(&self) -> bool {
        self.source
            .as_ref()
            .is_some_and(|s| s.draft != s.baseline || s.recovery.pending())
    }
    fn check_owner(&self, candidate: &ProjectSchema) -> Result<(), FileError> {
        check_owner(self.last_valid_schema(), candidate)
    }
    /// Last validated declarations; consult `output_status` before presenting them as current.
    pub fn last_valid_schema(&self) -> &ProjectSchema {
        self.schema_output.last_valid_schema()
    }
    pub fn last_valid_schema_snapshot(&self) -> Arc<ProjectSchema> {
        self.schema_output.last_valid_schema().clone()
    }
    pub fn output_status(&self) -> SchemaOutputStatus {
        self.schema_output.status()
    }
    pub fn current(&self) -> bool {
        self.source.as_ref().is_some_and(|source| {
            source_work::inspect_source(&self.schema_output, &source.path, &source.draft)
                .is_ok_and(|status| status == SourceStatus::Current)
        })
    }
}
fn check_owner(expected: &ProjectSchema, candidate: &ProjectSchema) -> Result<(), FileError> {
    let producer = |schema: &ProjectSchema| {
        schema
            .producer_metadata
            .as_ref()
            .and_then(|m| m.producer.clone())
    };
    let expected = producer(expected);
    if expected.is_none() || expected != producer(candidate) {
        return Err(FileError::SchemaOwnership);
    }
    Ok(())
}
fn parse(path: &Path, text: &str) -> Result<SchemaSource, FileError> {
    let loaded = SchemaSource::load_str(path.to_string_lossy(), text);
    loaded
        .source
        .ok_or(FileError::Validation(loaded.diagnostics))
}
#[cfg(test)]
mod tests;
