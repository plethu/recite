use super::linked::Control;
use recite_writer_session::ProjectSession;
use recite_writer_session::{project::SourceRepairArtifact, startup::EditorCommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[derive(Clone)]
pub enum ProjectNext {
    ReloadDeclarations,
    ImportDeclarations,
    ImportSettings,
    ReloadSettings,
    ImportMerge,
}

#[derive(Clone)]
pub enum Target {
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
pub struct Request {
    target: Target,
    pub editor: Option<EditorCommand>,
    pub link: Option<Arc<Control>>,
}
impl Request {
    pub fn merge_draft(
        project: &ProjectSession,
        path: PathBuf,
        editor: Option<EditorCommand>,
    ) -> Self {
        Self {
            target: Target::ProjectFile {
                root: project.root().to_owned(),
                path,
                next: ProjectNext::ImportMerge,
            },
            editor,
            link: None,
        }
    }
    pub fn declaration_source(
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
            link: None,
        }
    }
    pub fn declaration_draft(
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
            link: None,
        }
    }

    pub fn settings_draft(
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
            link: None,
        }
    }
    pub fn project_manifest(
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
            link: None,
        }
    }
    pub fn source(path: PathBuf, editor: Option<EditorCommand>) -> Self {
        Self {
            target: Target::Source(path),
            editor,
            link: None,
        }
    }
    pub fn repair(artifact: &SourceRepairArtifact, editor: Option<EditorCommand>) -> Self {
        Self {
            target: Target::Repair {
                document: artifact.document.clone(),
                path: artifact.source.clone(),
            },
            editor,
            link: None,
        }
    }
    pub fn with_system_editor(&self) -> Self {
        Self {
            target: self.target.clone(),
            editor: None,
            link: None,
        }
    }
    pub fn path(&self) -> &Path {
        match &self.target {
            Target::Source(path)
            | Target::Repair { path, .. }
            | Target::ProjectFile { path, .. } => path,
        }
    }
    pub fn check_link(&self) -> Result<(), String> {
        self.link.as_ref().map_or(Ok(()), |control| control.check())
    }
    pub fn finish_link(&self, result: Result<(), String>) {
        if let Some(control) = &self.link {
            control.complete(result);
        }
    }
    pub fn is_current(&self, project: &ProjectSession) -> bool {
        match &self.target {
            Target::Source(path) => project.current().path() == path,
            Target::Repair { document, .. } => project.current().path() == document,
            Target::ProjectFile { root, .. } => project.root() == root,
        }
    }
    pub fn is_separate_file(&self) -> bool {
        !matches!(self.target, Target::Source(_))
    }
    pub fn next_step(&self) -> &'static str {
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
            Target::ProjectFile {
                next: ProjectNext::ImportMerge,
                ..
            } => "Choose Import reviewed merge to validate its changes",
        }
    }
}
impl Request {
    /// Confirm the linked reply is still wanted, then hand the file to the configured editor.
    pub fn start(&self) -> Result<recite_writer_session::external::Handoff, String> {
        let result = self.check_link().and_then(|()| {
            recite_writer_session::external::Handoff::configured(
                self.path().to_owned(),
                self.editor.as_ref(),
            )
        });
        self.finish_link(result.as_ref().map(|_| ()).map_err(Clone::clone));
        result
    }
}
