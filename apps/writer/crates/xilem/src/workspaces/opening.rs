use super::*;
use recite_writer_session::{
    builds,
    declarations::{entries, session::Session},
    project::settings as settings_session,
};
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(crate) enum Open {
    Declarations,
    Build,
    Settings,
}
pub(crate) struct Work {
    root: PathBuf,
    action: Open,
}
pub(crate) enum Prepared {
    Declarations {
        root: PathBuf,
        session: Option<Box<Session>>,
        rows: Vec<declarations::Row>,
        producer: Vec<String>,
    },
    Build {
        root: PathBuf,
        targets: Vec<(String, Vec<String>)>,
    },
    Settings {
        root: PathBuf,
        session: Option<settings_session::Session>,
        preview: Vec<String>,
    },
}
impl Work {
    pub fn capture(writer: &Writer, action: Open) -> Result<Self, String> {
        let project = writer.session.project().ok_or("Open a project first.")?;
        if project.files().builds.busy()
            || project
                .files()
                .settings
                .as_ref()
                .is_some_and(|session| session.busy())
            || project
                .files()
                .declarations
                .as_ref()
                .is_some_and(|session| session.busy())
        {
            return Err("Wait for the current project job before refreshing its workspace.".into());
        }
        Ok(Self {
            root: project.root().to_owned(),
            action,
        })
    }
    pub fn execute(self) -> Result<Prepared, String> {
        match self.action {
            Open::Declarations => {
                let session = Session::open(&self.root).map_err(|error| error.to_string())?;
                let rows = entries::entries(session.last_valid_schema())
                    .into_iter()
                    .map(declarations::Row::new)
                    .collect();
                Ok(Prepared::Declarations {
                    producer: declarations::producer_description(&session.registration),
                    root: self.root,
                    session: Some(Box::new(session)),
                    rows,
                })
            }
            Open::Build => Ok(Prepared::Build {
                targets: builds::targets(&self.root)?,
                root: self.root,
            }),
            Open::Settings => {
                let session = settings_session::Session::open(&self.root)
                    .map_err(|error| error.to_string())?;
                let preview = text::pages(session.baseline());
                Ok(Prepared::Settings {
                    root: self.root,
                    session: Some(session),
                    preview,
                })
            }
        }
    }
}
impl Writer {
    pub(crate) fn accept_project_workspace(
        &mut self,
        prepared: &mut Prepared,
    ) -> Result<(), String> {
        let root = match prepared {
            Prepared::Declarations { root, .. }
            | Prepared::Build { root, .. }
            | Prepared::Settings { root, .. } => root,
        };
        let project = self
            .session
            .project_mut()
            .filter(|project| project.root() == root)
            .ok_or("The project changed while its workspace was opening.")?;
        let (files, _) = project.parts_mut();
        let screen = match prepared {
            Prepared::Declarations {
                session,
                rows,
                producer,
                ..
            } => {
                if files.declarations.is_none() {
                    files.declarations = session.take().map(|session| *session);
                    self.workspaces.declarations.replace(std::mem::take(rows));
                    self.workspaces
                        .declarations
                        .describe_producer(std::mem::take(producer));
                }
                Screen::Declarations
            }
            Prepared::Build { targets, .. } => {
                std::mem::swap(&mut files.builds.targets, targets);
                if files.builds.selected.as_ref().is_some_and(|selected| {
                    !files
                        .builds
                        .targets
                        .iter()
                        .any(|(asset, _)| asset == selected)
                }) {
                    files.builds.selected = None;
                }
                Screen::Build
            }
            Prepared::Settings {
                session, preview, ..
            } => {
                if files.settings.is_none() {
                    files.settings = session.take();
                    self.workspaces
                        .settings
                        .replace_preview(std::mem::take(preview));
                }
                Screen::Settings
            }
        };
        self.cancel_map_gesture();
        self.attention.leave_focus();
        self.workspaces.screen = screen;
        self.status = "Ready".into();
        Ok(())
    }
}
