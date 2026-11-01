//! Destination preparation shared by native adapters; publication retains the live owners.
use crate::{
    location::{self, Location},
    project::{PreparedSwitch, SwitchRequest},
};
use recite_writer_model::{NavigationRequest, PreparedNavigation, ReplyRules, Workbench};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum NavigationError {
    #[error("{0}")]
    Location(String),
    #[error(transparent)]
    File(#[from] crate::FileError),
    #[error(transparent)]
    Selection(#[from] recite_writer_model::WorkbenchError),
}

pub enum Destination {
    Current(Box<NavigationRequest>),
    Switch(Box<SwitchRequest>),
    Example(Box<crate::examples::SwitchRequest>),
}
pub enum PreparedDestination {
    Current(PreparedNavigation),
    Switch(PreparedSwitch),
    Example(Box<crate::examples::PreparedSwitch>),
}
pub struct Prepare {
    pub destination: Destination,
    pub location: Option<Location>,
    pub project: Option<PathBuf>,
    pub catalogue: Option<PathBuf>,
    pub reuse_rules: bool,
    pub initial_view: Option<recite_config::WriterView>,
}
pub struct Prepared {
    pub destination: PreparedDestination,
    pub rules: Option<ReplyRules>,
    pub catalogue: Option<PathBuf>,
    pub catalogue_changed: bool,
}
impl Prepare {
    pub fn execute(
        self,
        control: &dyn recite_compiler::authoring::WorkControl,
    ) -> Result<Prepared, NavigationError> {
        control.checkpoint().map_err(crate::FileError::from)?;
        let location = self.location.as_ref();
        if let Some(project) = location.and_then(|l| l.project.as_ref()) {
            // Already-open canonical roots need no repeated whole-project discovery.
            if self.project.as_deref() != Some(std::path::Path::new(project)) {
                let linked = recite_config::discover_project(project)
                    .map_err(|e| {
                        NavigationError::Location(format!("The linked project is unavailable: {e}"))
                    })?
                    .manifest()
                    .project_root()
                    .to_owned();
                if self.project.as_ref() != Some(&linked) {
                    return Err(NavigationError::Location(format!(
                        "Open the linked project first: {project}"
                    )));
                }
            }
        }
        let path = location
            .and_then(|l| l.catalogue.as_ref())
            .map(|path| {
                self.project.as_ref().map_or_else(
                    || Ok(PathBuf::from(path)),
                    |root| location::catalogue_path(root, path),
                )
            })
            .transpose()
            .map_err(NavigationError::Location)?;
        let catalogue_changed = location.is_some() && path != self.catalogue;
        let catalogue = if catalogue_changed { path } else { None };
        let mut rules = None;
        let mut select =
            |model: &mut Workbench| -> Result<(), recite_writer_model::WorkbenchError> {
                if let Some(location) = location {
                    if self.reuse_rules {
                        location::select_reusing_rules(model, location)?;
                    } else {
                        rules = location::select(model, location)?;
                    }
                } else if let Some(view) = self.initial_view
                    && !model.has_draft()
                {
                    if view == recite_config::WriterView::Source {
                        model.select(recite_writer_model::View::Source)?;
                    } else {
                        model.show_script()?;
                    }
                }
                Ok(())
            };
        let destination = match self.destination {
            Destination::Current(request) => {
                let mut prepared = request.prepare(None, control)?;
                select(prepared.model_mut())?;
                PreparedDestination::Current(prepared)
            }
            Destination::Switch(request) => {
                PreparedDestination::Switch(request.prepare(control, select)?)
            }
            Destination::Example(request) => {
                PreparedDestination::Example(Box::new(request.prepare(control, select)?))
            }
        };
        Ok(Prepared {
            destination,
            rules,
            catalogue,
            catalogue_changed,
        })
    }
}
