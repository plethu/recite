//! Explicit standalone source association; generated manifests are never edited.
use crate::project::{FileError, read_regular, save::replace_checked};
use recite_core::schema::{ProjectSchema, SchemaSource};
use std::path::{Path, PathBuf};

pub(crate) struct Session {
    pub output: PathBuf,
    pub schema: ProjectSchema,
    output_baseline: String,
    pub source: Option<Source>,
    pub registration: Result<Option<recite_config::ProducerRegistration>, String>,
    pub(super) job: Option<super::producer::Job>,
    root: PathBuf,
}
pub(crate) struct Source {
    pub path: PathBuf,
    pub(super) baseline: String,
    pub(super) recovery: super::recovery::Store,
    pub draft: String,
    pub(super) revision: u64,
}
impl Session {
    pub fn open(root: &Path) -> Result<Self, FileError> {
        let report = recite_config::discover_project(root)?;
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
        let registration = Self::registration(root, &schema);
        Ok(Self {
            registration,
            job: None,
            root: root.to_owned(),
            output,
            schema,
            output_baseline,
            source: None,
        })
    }
    fn registration(
        root: &Path,
        schema: &ProjectSchema,
    ) -> Result<Option<recite_config::ProducerRegistration>, String> {
        let path = root.join(recite_config::PRODUCER_REGISTRATION_FILE);
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let registration =
            recite_config::ProducerRegistration::parse(&text).map_err(|e| e.to_string())?;
        if schema
            .producer_metadata
            .as_ref()
            .and_then(|m| m.producer.as_ref())
            != Some(registration.producer())
        {
            return Err("Registration belongs to a different schema producer.".into());
        }
        Ok(Some(registration))
    }
    pub(crate) fn matches_schema(&self, relative: Option<&str>) -> bool {
        relative.is_some_and(|path| self.root.join(path) == self.output)
    }
    pub(crate) fn check_project_settings(&self, source: &str) -> Result<(), FileError> {
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
        self.schema = schema;
        self.output_baseline = text;
        Ok(())
    }
    pub fn reload_registration(&mut self) {
        if self.job.is_none() {
            self.registration = Self::registration(&self.root, &self.schema);
        }
    }
    pub fn start_generation(&mut self) -> Result<(), String> {
        if self.job.is_some() {
            return Err("Generation is already running.".into());
        }
        if self.dirty() {
            return Err("Save or discard the standalone source draft first.".into());
        }
        let registration = self
            .registration
            .as_ref()
            .map_err(Clone::clone)?
            .as_ref()
            .ok_or("No producer registered.")?;
        // Run only the configuration displayed on this screen.
        let current = Self::registration(&self.root, &self.schema)?;
        if current.as_ref() != Some(registration) {
            return Err(
                "Producer registration changed. Reload it and review the command before retrying."
                    .into(),
            );
        }
        self.job = Some(super::producer::Job::start(
            self.root.clone(),
            registration.clone(),
        )?);
        Ok(())
    }
    pub(super) fn finish_generation(
        &mut self,
        result: Result<super::producer::Generated, String>,
    ) -> Result<(), String> {
        self.job = None;
        result.and_then(|generated| {
            replace_checked(&self.output, &self.output_baseline, &generated.text)
                .map_err(|e| e.to_string())?;
            self.output_baseline = generated.text;
            self.schema = generated.schema;
            Ok(())
        })
    }
    pub(crate) fn cancel(&self) {
        if let Some(job) = &self.job {
            job.cancel();
        }
    }
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
    pub fn dirty(&self) -> bool {
        self.source.as_ref().is_some_and(|s| s.draft != s.baseline)
    }
    pub fn bind(&mut self, path: &Path) -> Result<(), FileError> {
        if self.busy() {
            return Err(FileError::UnsavedDocument);
        }
        if self.dirty() {
            return Err(FileError::UnsavedDocument);
        }
        if path.canonicalize()? == self.output.canonicalize()? {
            return Err(FileError::SchemaOwnership);
        }
        let baseline = read_regular(path)?;
        let source = parse(path, &baseline)?;
        self.check_owner(source.schema())?;
        let recovery = super::recovery::Store::open(path)?;
        let (baseline, draft) = recovery
            .snapshot()
            .map(|s| (s.baseline.clone(), s.draft.clone()))
            .unwrap_or_else(|| (baseline.clone(), baseline));
        self.check_owner(parse(path, &baseline)?.schema())?;
        self.source = Some(Source {
            path: path.to_owned(),
            baseline,
            draft,
            recovery,
            revision: 0,
        });
        Ok(())
    }
    fn check_owner(&self, candidate: &ProjectSchema) -> Result<(), FileError> {
        let producer = |s: &ProjectSchema| {
            s.producer_metadata
                .as_ref()
                .and_then(|m| m.producer.clone())
        };
        let expected = producer(&self.schema);
        if expected.is_none() || expected != producer(candidate) {
            return Err(FileError::SchemaOwnership);
        }
        Ok(())
    }
    /// Save the validated source first. If publishing fails, the old output and
    /// the saved source remain available and generation can be retried.
    pub fn save_and_generate(&mut self) -> Result<(), FileError> {
        if self.busy() {
            return Err(FileError::UnsavedDocument);
        }
        let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
        let parsed = parse(&source.path, &source.draft)?;
        self.check_owner(parsed.schema())?;
        // Refuse known stale output before saving source, then check again at publication.
        if read_regular(&self.output)? != self.output_baseline {
            return Err(FileError::Conflict);
        }
        let source = self.source.as_mut().ok_or(FileError::SchemaOwnership)?;
        replace_checked(&source.path, &source.baseline, &source.draft)?;
        source.baseline = source.draft.clone();
        source.flush_recovery()?;
        let generated = parsed.export_json();
        replace_checked(&self.output, &self.output_baseline, &generated)?;
        self.output_baseline = generated;
        self.schema = parsed.schema().clone();
        Ok(())
    }
    /// Explicit reload keeps the previous draft as a source file before rebasing.
    pub fn reload_source(&mut self) -> Result<PathBuf, FileError> {
        use std::io::Write;
        let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
        let disk = read_regular(&source.path)?;
        let output = read_regular(&self.output)?;
        let loaded =
            recite_core::schema::load_schema_manifest_str(self.output.to_string_lossy(), &output);
        if let Some(schema) = &loaded.schema {
            self.check_owner(schema)?;
        }
        let mut copy = tempfile::Builder::new()
            .prefix(".recite-schema-recovered-")
            .suffix(".toml")
            .tempfile_in(source.path.parent().ok_or(FileError::Selection)?)?;
        copy.write_all(source.draft.as_bytes())?;
        copy.as_file().sync_all()?;
        let (_, path) = copy.keep().map_err(|e| e.error)?;
        if let Some(source) = &mut self.source {
            source.baseline = disk.clone();
            source.draft = disk;
            source.revision = source.revision.wrapping_add(1);
            source.flush_recovery()?;
        }
        self.output_baseline = output;
        if let Some(schema) = loaded.schema {
            self.schema = schema;
        }
        Ok(path)
    }
    pub fn discard(&mut self) -> Result<(), FileError> {
        if let Some(source) = &mut self.source {
            source.draft = source.baseline.clone();
            source.revision = source.revision.wrapping_add(1);
            source.flush_recovery()?;
        }
        Ok(())
    }
    pub fn current(&self) -> bool {
        self.source.as_ref().is_some_and(|s| {
            parse(&s.path, &s.draft).is_ok_and(|source| {
                source.schema_fingerprint()
                    == recite_core::schema::canonical_schema_fingerprint(&self.schema)
            })
        })
    }
}
fn parse(path: &Path, text: &str) -> Result<SchemaSource, FileError> {
    let loaded = SchemaSource::load_str(path.to_string_lossy(), text);
    loaded
        .source
        .ok_or(FileError::Validation(loaded.diagnostics))
}
#[cfg(test)]
mod tests;
