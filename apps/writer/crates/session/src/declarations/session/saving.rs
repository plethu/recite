//! Source and generated output publish in order, retaining successful prefixes.
use super::{FileError, PathBuf, ProjectSchema, Session, parse, read_regular, replace_checked};
impl Session {
    pub fn prepare_save(&self) -> Result<DeclarationSave, FileError> {
        if self.busy() {
            return Err(FileError::UnsavedDocument);
        }
        let source = self.source.as_ref().ok_or(FileError::SchemaOwnership)?;
        let parsed = parse(&source.path, &source.draft)?;
        self.check_owner(parsed.schema())?;
        Ok(DeclarationSave {
            source_path: source.path.clone(),
            baseline: source.baseline.clone(),
            source: source.draft.clone(),
            output: self.output.clone(),
            output_baseline: self.output_baseline.clone(),
            generated: parsed.export_json(),
            schema: parsed.schema().clone(),
        })
    }
    pub fn accept_saved_source(
        &mut self,
        request: &DeclarationSave,
    ) -> Result<
        crate::recovery::worker::Durability<crate::declarations::recovery::DraftSnapshot>,
        FileError,
    > {
        let source = self.source.as_mut().ok_or(FileError::SchemaOwnership)?;
        source.baseline = request.source.clone();
        source.begin_saved_recovery()
    }
    pub fn accept_saved_output(&mut self, request: DeclarationSave) {
        self.output_baseline = request.generated;
        self.schema = request.schema;
    }
}
/// Captured standalone source and generated output share one validation result.
pub struct DeclarationSave {
    source_path: PathBuf,
    baseline: String,
    source: String,
    output: PathBuf,
    output_baseline: String,
    generated: String,
    schema: ProjectSchema,
}
impl DeclarationSave {
    pub fn write_source(&self) -> Result<(), FileError> {
        if read_regular(&self.output)? != self.output_baseline {
            return Err(FileError::Conflict);
        }
        replace_checked(&self.source_path, &self.baseline, &self.source)
    }
    pub fn write_output(&self) -> Result<(), FileError> {
        replace_checked(&self.output, &self.output_baseline, &self.generated)
    }
}

impl Session {
    pub fn save_and_generate(&mut self) -> Result<(), FileError> {
        self.require_idle_save()?;
        let request = self.prepare_save()?;
        request.write_source()?;
        self.accept_saved_source(&request)?.wait()?;
        request.write_output()?;
        self.accept_saved_output(request);
        Ok(())
    }
}
