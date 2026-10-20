use recite_writer_session::{project::SourceRepairArtifact, startup::EditorCommand};
use std::path::{Path, PathBuf};

#[derive(Clone)]
enum Target {
    Source(PathBuf),
    Repair { document: PathBuf, path: PathBuf },
}
pub(crate) struct Request {
    target: Target,
    pub(super) editor: Option<EditorCommand>,
}
impl Request {
    pub(super) fn source(path: PathBuf, editor: Option<EditorCommand>) -> Self {
        Self {
            target: Target::Source(path),
            editor,
        }
    }
    pub(super) fn repair(artifact: &SourceRepairArtifact, editor: Option<EditorCommand>) -> Self {
        Self {
            target: Target::Repair {
                document: artifact.document.clone(),
                path: artifact.source.clone(),
            },
            editor,
        }
    }
    pub(super) fn with_system_editor(&self) -> Self {
        Self {
            target: self.target.clone(),
            editor: None,
        }
    }
    pub(crate) fn path(&self) -> &Path {
        match &self.target {
            Target::Source(path) | Target::Repair { path, .. } => path,
        }
    }
    pub(super) fn document(&self) -> &Path {
        match &self.target {
            Target::Source(path) => path,
            Target::Repair { document, .. } => document,
        }
    }
    pub(super) fn is_repair(&self) -> bool {
        matches!(self.target, Target::Repair { .. })
    }
}
