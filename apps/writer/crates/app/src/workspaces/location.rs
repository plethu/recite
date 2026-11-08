//! Detached preparation for linked project screens, before any presentation is replaced.
pub use super::{Open, Prepared, Screen, Work, Writer, external::Inspection};
use recite_compiler::authoring::WorkControl;
use recite_writer_session::{
    ProjectSession, location::Screen as LinkedScreen, project::ExternalComparison,
};

pub(crate) enum Plan {
    Writing,
    Workspace(Work),
    Disk,
}
pub struct Presentation(Surface);
enum Surface {
    Writing,
    Workspace(Prepared),
    Disk(Box<Disk>),
}
struct Disk {
    comparison: Option<ExternalComparison>,
    inspection: Option<Inspection>,
}
impl Plan {
    pub fn capture(writer: &Writer, screen: LinkedScreen) -> Result<Self, String> {
        let action = match screen {
            LinkedScreen::Declarations => Open::Declarations,
            LinkedScreen::Build => Open::Build,
            LinkedScreen::Rename => Open::Rename,
            LinkedScreen::Disk => return Ok(Self::Disk),
            LinkedScreen::Write
            | LinkedScreen::Preview
            | LinkedScreen::Rules
            | LinkedScreen::Localise
            | LinkedScreen::Translations
            | LinkedScreen::Updates
            | LinkedScreen::Compare
            | LinkedScreen::Entry => return Ok(Self::Writing),
        };
        Work::capture(writer, action).map(Self::Workspace)
    }
    pub fn prepare(
        self,
        project: &ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<Presentation, String> {
        control.checkpoint().map_err(|error| error.to_string())?;
        match self {
            Self::Writing => Ok(Presentation(Surface::Writing)),
            Self::Workspace(work) => work
                .execute()
                .map(|prepared| Presentation(Surface::Workspace(prepared))),
            Self::Disk => {
                let comparison = project
                    .files()
                    .external_comparison(project.current().model())
                    .map_err(|error| error.to_string())?;
                let inspection = Inspection::prepare(&comparison);
                Ok(Presentation(Surface::Disk(Box::new(Disk {
                    comparison: Some(comparison),
                    inspection: Some(inspection),
                }))))
            }
        }
    }
}
impl Writer {
    pub(crate) fn present_linked_workspace(
        &mut self,
        presentation: &mut Presentation,
    ) -> Result<(), String> {
        match &mut presentation.0 {
            Surface::Writing => self.workspaces.screen = Screen::Write,
            Surface::Workspace(workspace) => self.accept_project_workspace(workspace)?,
            Surface::Disk(disk) => {
                let (files, _) = self
                    .session
                    .parts_mut()
                    .ok_or("The project owner is unavailable.")?;
                std::mem::swap(&mut files.external, &mut disk.comparison);
                self.workspaces.external.exchange(&mut disk.inspection);
                self.workspaces.screen = Screen::External;
            }
        }
        Ok(())
    }
}
