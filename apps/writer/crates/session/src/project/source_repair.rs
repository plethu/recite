//! Export the staged Source field as text, with its complete recovery backup.
use super::{FileError, ProjectFiles};
use crate::{files::sync_directory, recovery::Recovery};
use recite_writer_model::{View, Workbench};
use std::{io::Write, path::PathBuf};

pub struct SourceRepairExport {
    path: PathBuf,
    recovery: Recovery,
}
#[derive(Clone, Debug)]
pub struct SourceRepairArtifact {
    pub document: PathBuf,
    pub source: PathBuf,
    pub recovery: PathBuf,
}
impl ProjectFiles {
    /// Capturing an export shares immutable text and performs no disk operations.
    pub fn source_repair_export(&self, model: &Workbench) -> Result<SourceRepairExport, FileError> {
        if model.view() != &View::Source {
            return Err(FileError::Selection);
        }
        Ok(SourceRepairExport {
            path: self.current.clone(),
            recovery: Recovery::new(self.saved.clone(), model.recovery()),
        })
    }
}
impl SourceRepairExport {
    /// Run on a worker. Kept files are user recovery artifacts, never app scratch.
    pub fn write(self) -> Result<SourceRepairArtifact, FileError> {
        let parent = self.path.parent().ok_or(FileError::Selection)?;
        let recovery = crate::export_recovery(&self.path, &self.recovery)?;
        // A .txt suffix keeps this editable artifact out of recursive .recite discovery.
        let mut source = tempfile::Builder::new()
            .prefix("recite-source-repair-")
            .suffix(".txt")
            .tempfile_in(parent)?;
        write!(source, "{}", self.recovery.draft.draft_snapshot().as_str())?;
        source.as_file().sync_all()?;
        let (_, source) = source.keep().map_err(|error| error.error)?;
        sync_directory(parent)?;
        Ok(SourceRepairArtifact {
            document: self.path,
            source,
            recovery,
        })
    }
}
