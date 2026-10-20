use recite_writer_session::ProjectSession;
use recite_writer_session::{project::SourceRepairArtifact, startup::EditorCommand};
use std::path::{Path, PathBuf};
#[derive(Clone)]
enum ProjectNext {
    ReloadDeclarations,
    ImportDeclarations,
    ImportSettings,
    ReloadSettings,
}

#[derive(Clone)]
enum Target {
    Source(PathBuf),
    Repair {
        document: PathBuf,
        path: PathBuf,
    },
    ProjectFile {
        root: PathBuf,
        path: PathBuf,
        next: ProjectNext,
    },
}
pub(crate) struct Request {
    target: Target,
    pub(super) editor: Option<EditorCommand>,
}
impl Request {
    pub(crate) fn declaration_source(
        project: &ProjectSession,
        path: PathBuf,
        editor: Option<EditorCommand>,
    ) -> Self {
        Self {
            target: Target::ProjectFile {
                root: project.root().to_owned(),
                path,
                next: ProjectNext::ReloadDeclarations,
            },
            editor,
        }
    }
    pub(crate) fn declaration_draft(
        project: &ProjectSession,
        path: PathBuf,
        editor: Option<EditorCommand>,
    ) -> Self {
        Self {
            target: Target::ProjectFile {
                root: project.root().to_owned(),
                path,
                next: ProjectNext::ImportDeclarations,
            },
            editor,
        }
    }

    pub(crate) fn settings_draft(
        project: &ProjectSession,
        path: PathBuf,
        editor: Option<EditorCommand>,
    ) -> Self {
        Self {
            target: Target::ProjectFile {
                root: project.root().to_owned(),
                path,
                next: ProjectNext::ImportSettings,
            },
            editor,
        }
    }
    pub(crate) fn project_manifest(
        project: &ProjectSession,
        path: PathBuf,
        editor: Option<EditorCommand>,
    ) -> Self {
        Self {
            target: Target::ProjectFile {
                root: project.root().to_owned(),
                path,
                next: ProjectNext::ReloadSettings,
            },
            editor,
        }
    }
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
            Target::Source(path)
            | Target::Repair { path, .. }
            | Target::ProjectFile { path, .. } => path,
        }
    }
    pub(super) fn is_current(&self, project: &ProjectSession) -> bool {
        match &self.target {
            Target::Source(path) => project.current().path() == path,
            Target::Repair { document, .. } => project.current().path() == document,
            Target::ProjectFile { root, .. } => project.root() == root,
        }
    }
    pub(super) fn is_separate_file(&self) -> bool {
        !matches!(self.target, Target::Source(_))
    }
    pub(super) fn next_step(&self) -> &'static str {
        match &self.target {
            Target::Source(_) => "Reload to inspect external changes",
            Target::Repair { .. } => "Choose Import repaired source to validate its changes",
            Target::ProjectFile {
                next: ProjectNext::ReloadDeclarations,
                ..
            } => "Reload declaration source to inspect its changes",
            Target::ProjectFile {
                next: ProjectNext::ImportDeclarations,
                ..
            } => "Choose Import repaired declaration draft to validate its changes",
            Target::ProjectFile {
                next: ProjectNext::ImportSettings,
                ..
            } => "Choose Import settings draft to validate its changes",
            Target::ProjectFile {
                next: ProjectNext::ReloadSettings,
                ..
            } => "Choose Reload saved settings to inspect its changes",
        }
    }
}
