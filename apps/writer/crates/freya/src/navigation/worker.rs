//! Own destination preparation, filesystem preflight and rejected-result retirement.
use super::{apply, location::Location};
use crate::project::{Checkpoint, PreparedSwitch, ReadyCheckpoint, SwitchRequest};
use recite_writer_model::{NavigationRequest, PreparedNavigation, ReplyRules, Workbench};
use std::path::PathBuf;

pub(crate) enum Destination {
    Current(NavigationRequest),
    Switch(Box<SwitchRequest>),
    Example {
        name: String,
        retained: Option<NavigationRequest>,
    },
}
pub(crate) enum PreparedDestination {
    Current(PreparedNavigation),
    Switch(PreparedSwitch),
    Example { name: String, prepared: Example },
}
pub(crate) enum Example {
    Retained(Box<PreparedNavigation>),
    Cold(Option<Box<Workbench>>),
}
pub(crate) struct Prepare {
    pub destination: Destination,
    pub location: Option<Location>,
    pub project: Option<PathBuf>,
    pub catalogue: Option<PathBuf>,
    pub reuse_rules: bool,
    pub initial_view: Option<recite_config::WriterView>,
}
pub(crate) struct Prepared {
    pub destination: PreparedDestination,
    pub rules: Option<ReplyRules>,
    pub catalogue: Option<PathBuf>,
    pub catalogue_changed: bool,
}
pub(crate) enum Work {
    Prepare(Box<Prepare>),
    Checkpoint(Box<Checkpoint>),
    RetireRequest(Box<super::request::Navigation>),
}
pub(crate) enum Output {
    Prepared(Result<Box<Prepared>, String>),
    Checkpoint(Result<ReadyCheckpoint, String>),
    Retired,
}
impl Work {
    pub(crate) fn execute(self) -> Output {
        match self {
            Self::Prepare(request) => Output::Prepared(request.execute().map(Box::new)),
            Self::Checkpoint(request) => {
                Output::Checkpoint(request.wait().map_err(|e| e.to_string()))
            }
            Self::RetireRequest(request) => {
                drop(request);
                Output::Retired
            }
        }
    }
}
impl Prepare {
    fn execute(self) -> Result<Prepared, String> {
        let location = self.location.as_ref();
        if let Some(project) = location.and_then(|l| l.project.as_ref()) {
            // Already-open canonical roots need no repeated whole-project discovery.
            if self.project.as_deref() != Some(std::path::Path::new(project)) {
                let linked = recite_config::discover_project(project)
                    .map_err(|e| format!("The linked project is unavailable: {e}"))?
                    .manifest()
                    .project_root()
                    .to_owned();
                if self.project.as_ref() != Some(&linked) {
                    return Err(format!("Open the linked project first: {project}"));
                }
            }
        }
        let path = location
            .and_then(|l| l.catalogue.as_ref())
            .map(|path| {
                self.project.as_ref().map_or_else(
                    || Ok(PathBuf::from(path)),
                    |root| apply::catalogue_path(root, path),
                )
            })
            .transpose()?;
        let catalogue_changed = location.is_some() && path != self.catalogue;
        let catalogue = if catalogue_changed { path } else { None };
        let mut rules = None;
        let mut select =
            |model: &mut Workbench| -> Result<(), recite_writer_model::WorkbenchError> {
                if let Some(location) = location {
                    if self.reuse_rules {
                        apply::select_reusing_rules(model, location)?;
                    } else {
                        rules = apply::select(model, location)?;
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
        let control = recite_compiler::authoring::CancellationToken::new();
        let destination = match self.destination {
            Destination::Current(request) => {
                let mut prepared = request.prepare(None, &control).map_err(|e| e.to_string())?;
                select(prepared.model_mut()).map_err(|e| e.to_string())?;
                PreparedDestination::Current(prepared)
            }
            Destination::Switch(request) => PreparedDestination::Switch(
                request
                    .prepare(&control, select)
                    .map_err(|e| e.to_string())?,
            ),
            Destination::Example { name, retained } => {
                let prepared = if let Some(request) = retained {
                    let mut prepared =
                        request.prepare(None, &control).map_err(|e| e.to_string())?;
                    select(prepared.model_mut()).map_err(|e| e.to_string())?;
                    Example::Retained(Box::new(prepared))
                } else {
                    let example = recite_writer_model::WRITER_EXAMPLES
                        .iter()
                        .find(|e| e.name == name)
                        .ok_or("The linked scene is not open.")?;
                    let mut model = example.open().map_err(|e| e.to_string())?;
                    select(&mut model).map_err(|e| e.to_string())?;
                    Example::Cold(Some(Box::new(model)))
                };
                PreparedDestination::Example { name, prepared }
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
