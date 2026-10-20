//! Prepared source facts share one schema owner, independently of widget presentation.
use super::super::{check_owner, parse};
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceStatus {
    Current,
    NeedsGeneration,
    DamagedOutput,
}
impl DeclarationReceipt {
    /// Worker-side validation also describes recovered drafts that cannot yet be saved.
    pub fn inspect_source(&self) -> Result<SourceStatus, FileError> {
        let draft = self.draft().ok_or(FileError::SchemaOwnership)?;
        let path = match self.result.as_ref().and_then(|result| result.as_ref().ok()) {
            Some(PreparedDeclaration::Bound(source)) => &source.path,
            _ => &self.inputs.source().ok_or(FileError::SchemaOwnership)?.path,
        };
        inspect_source(
            self.output().ok_or(FileError::SchemaOwnership)?,
            path,
            draft,
        )
    }
    pub fn inspects_source(&self) -> bool {
        matches!(&self.inputs, Inputs::Source(_))
    }
    pub fn registration(
        &self,
    ) -> Option<&Result<Option<recite_config::ProducerRegistration>, String>> {
        match self.result.as_ref()?.as_ref().ok()? {
            PreparedDeclaration::Registration(registration) => Some(registration),
            _ => None,
        }
    }
    pub fn archive(&self) -> Option<&Path> {
        match &self.artifact {
            Some(SourceArtifact::Archived(path)) => Some(path),
            _ => None,
        }
    }
    pub fn editable_draft(&self) -> Option<&Path> {
        match &self.artifact {
            Some(SourceArtifact::Editable(path)) => Some(path),
            _ => None,
        }
    }
    pub fn draft(&self) -> Option<&str> {
        match self.result.as_ref()?.as_ref().ok()? {
            PreparedDeclaration::Bound(source) => Some(&source.draft),
            PreparedDeclaration::Replaced(state) => Some(&state.draft),
            PreparedDeclaration::Output(_) | PreparedDeclaration::Registration(_) => None,
            PreparedDeclaration::Unchanged => {
                self.inputs.source().map(|source| source.draft.as_str())
            }
        }
    }
    pub fn last_valid_schema(&self) -> Option<&ProjectSchema> {
        self.output()
            .map(|output| output.last_valid_schema().as_ref())
    }
    fn output(&self) -> Option<&SchemaOutput> {
        match self.result.as_ref()?.as_ref().ok()? {
            PreparedDeclaration::Replaced(state) => state
                .output
                .as_ref()
                .map(Arc::as_ref)
                .or(Some(self.schema_output.as_ref())),
            PreparedDeclaration::Output(output) => Some(output),
            _ => Some(self.schema_output.as_ref()),
        }
    }
    pub fn reloads_output(&self) -> bool {
        matches!(&self.result, Some(Ok(PreparedDeclaration::Output(_))))
            || matches!(&self.result, Some(Ok(PreparedDeclaration::Replaced(state))) if state.output.is_some())
    }
    pub fn replaces_source(&self) -> bool {
        matches!(
            &self.result,
            Some(Ok(
                PreparedDeclaration::Bound(_) | PreparedDeclaration::Replaced(_)
            ))
        )
    }
}
pub(in crate::declarations::session) fn inspect_source(
    output: &SchemaOutput,
    path: &Path,
    draft: &str,
) -> Result<SourceStatus, FileError> {
    let parsed = parse(path, draft)?;
    let schema = output.last_valid_schema();
    check_owner(schema, parsed.schema())?;
    if output.status() == SchemaOutputStatus::Damaged {
        return Ok(SourceStatus::DamagedOutput);
    }
    Ok(
        if parsed.schema_fingerprint() == recite_core::schema::canonical_schema_fingerprint(schema)
        {
            SourceStatus::Current
        } else {
            SourceStatus::NeedsGeneration
        },
    )
}
