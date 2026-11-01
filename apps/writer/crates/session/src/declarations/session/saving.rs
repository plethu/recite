//! Source and generated output publish in order, retaining successful prefixes.
use super::{
    Arc, FileError, PathBuf, SchemaOutput, Session, check_owner, parse, read_regular,
    replace_checked,
};
use crate::declarations::{producer::Generated, recovery};
use crate::recovery::worker;
impl Session {
    pub fn prepare_save(&self) -> Result<DeclarationSave, FileError> {
        self.save_request()?.prepare()
    }
    /// Capture only; parsing and export belong to the preparation worker.
    pub fn save_request(&self) -> Result<DeclarationSaveRequest, FileError> {
        if self.busy() {
            return Err(FileError::UnsavedDocument);
        }
        let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
        Ok(DeclarationSaveRequest {
            source_path: source.path.clone(),
            baseline: source.baseline.clone(),
            source: source.draft.clone(),
            output: self.output.clone(),
            expected_output: self.schema_output.clone(),
        })
    }
    pub fn accept_saved_source(
        &mut self,
        request: &DeclarationSave,
    ) -> Result<worker::Durability<recovery::DraftSnapshot>, FileError> {
        let source = self.source.as_mut().ok_or(FileError::SchemaOwnership)?;
        source.baseline = request.source.clone();
        source.begin_saved_recovery()
    }
    pub fn accept_saved_output(&mut self, request: &mut DeclarationSave) -> Result<(), FileError> {
        if !Arc::ptr_eq(&self.schema_output, &request.expected_output) {
            return Err(FileError::DeclarationChanged);
        }
        let output = request
            .prepared_output
            .take()
            .ok_or(FileError::DeclarationChanged)?;
        request.retired_output = Some(std::mem::replace(&mut self.schema_output, output));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
pub struct DeclarationSaveRequest {
    source_path: PathBuf,
    baseline: String,
    source: String,
    output: PathBuf,
    expected_output: Arc<SchemaOutput>,
}
impl DeclarationSaveRequest {
    pub fn prepare(self) -> Result<DeclarationSave, FileError> {
        let parsed = parse(&self.source_path, &self.source)?;
        check_owner(self.expected_output.last_valid_schema(), parsed.schema())?;
        Ok(DeclarationSave {
            source_path: self.source_path,
            baseline: self.baseline,
            source: self.source,
            output: self.output,
            expected_output: self.expected_output,
            prepared_output: Some(Arc::new(SchemaOutput::Ready(Generated {
                text: parsed.export_json(),
                schema: parsed.schema().clone().into(),
            }))),
            retired_output: None,
        })
    }
}
/// Captured standalone source and generated output share one validation result.
pub struct DeclarationSave {
    source_path: PathBuf,
    baseline: String,
    source: String,
    output: PathBuf,
    expected_output: Arc<SchemaOutput>,
    prepared_output: Option<Arc<SchemaOutput>>,
    retired_output: Option<Arc<SchemaOutput>>,
}
impl DeclarationSave {
    pub fn write_source(&self) -> Result<(), FileError> {
        if read_regular(&self.output)? != self.expected_output.baseline() {
            return Err(FileError::Conflict);
        }
        replace_checked(&self.source_path, &self.baseline, &self.source)
    }
    pub fn write_output(&self) -> Result<(), FileError> {
        let output = self
            .prepared_output
            .as_ref()
            .ok_or(FileError::DeclarationChanged)?;
        replace_checked(
            &self.output,
            self.expected_output.baseline(),
            output.baseline(),
        )
    }
}

impl Session {
    pub fn save_and_generate(&mut self) -> Result<(), FileError> {
        self.require_idle_save()?;
        let mut request = self.prepare_save()?;
        request.write_source()?;
        self.accept_saved_source(&request)?.wait()?;
        request.write_output()?;
        self.accept_saved_output(&mut request)
    }
}
