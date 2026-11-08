//! Only the worker loads placement, prepares synopsis, geometry and drawing paths.
use super::*;
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{AnalysisStamp, NavigationRequest};
use recite_writer_session::map::placement::{load, save};
use recite_writer_session::map::synopsis;

#[derive(Clone, Copy)]
pub enum Purpose {
    Workspace,
    Placement,
}
pub struct Preparation {
    pub purpose: Purpose,
    pub persist: bool,
    pub change: Option<placement::Change>,
    pub status: placement::Status,
    pub model: NavigationRequest,
    pub key: DocumentKey,
    pub stamp: AnalysisStamp,
    pub scene: String,
    pub store: Option<Result<recite_config::UserStateFile, String>>,
    pub positions: Option<Arc<Positions>>,
    pub previous: Option<Arc<Topology>>,
}
pub struct Prepared {
    pub key: DocumentKey,
    pub stamp: AnalysisStamp,
    pub change: Option<placement::Change>,
    pub persistence: Option<Result<(), String>>,
    pub next: Option<Projection>,
    pub retired: Option<Projection>,
    pub positions: Option<Arc<Positions>>,
    pub navigation: Option<recite_writer_model::PreparedNavigation>,
}
impl Preparation {
    pub fn execute(self, control: &dyn WorkControl) -> Result<Prepared, String> {
        let mut model = self
            .model
            .prepare(None, control)
            .map_err(|error| crate::feedback::workbench(&error))?;
        if matches!(self.purpose, Purpose::Workspace)
            && model.model().view() == &recite_writer_model::View::Source
            && !model.model().has_draft()
        {
            model
                .model_mut()
                .show_script()
                .map_err(|error| crate::feedback::workbench(&error))?;
        }
        let blocks = model
            .model()
            .document()
            .script_snapshot()
            .map_err(|error| crate::feedback::edit(&error))?;
        let mut status = self.status;
        let positions_origin = self.positions.clone();
        let positions = match self.positions {
            Some(positions) => positions,
            None => Arc::new(match &self.store {
                Some(Ok(file)) => match load(file, &self.scene) {
                    Ok(positions) => {
                        status = placement::Status::Ready;
                        positions
                    }
                    Err(message) => {
                        status = placement::Status::LoadFailed(message);
                        Positions::new()
                    }
                },
                Some(Err(message)) => {
                    status = placement::Status::LoadFailed(message.clone());
                    Positions::new()
                }
                None => Positions::new(),
            }),
        };
        let positions = if let Some(change) = &self.change {
            let mut next = positions.as_ref().clone();
            change.apply(&mut next);
            Arc::new(next)
        } else {
            positions
        };
        control.checkpoint().map_err(|error| error.to_string())?;
        let graph = Arc::new(Topology::refresh(
            blocks,
            &positions,
            self.previous.as_deref(),
        ));
        let synopsis = graph
            .blocks
            .iter()
            .map(|block| synopsis::synopsis(&block.entries))
            .collect();
        let paths = graph
            .routes
            .iter()
            .map(|route| {
                route
                    .as_ref()
                    .map(|route| {
                        Ok((
                            BezPath::from_svg(&route.path).map_err(|error| error.to_string())?,
                            BezPath::from_svg(&route.arrow).map_err(|error| error.to_string())?,
                        ))
                    })
                    .transpose()
            })
            .collect::<Result<Vec<_>, String>>()?;
        control.checkpoint().map_err(|error| error.to_string())?;
        // No fallible preparation or cancellation point may discard a completed write.
        let persistence = self.persist.then(|| {
            self.store.as_ref().map_or(Ok(()), |store| {
                store
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|file| save(file, &self.scene, &positions))
            })
        });
        if let Some(saved) = &persistence {
            status = match saved {
                Ok(()) => placement::Status::Ready,
                Err(message) => placement::Status::SaveFailed(message.clone()),
            };
        }
        Ok(Prepared {
            persistence,
            navigation: matches!(self.purpose, Purpose::Workspace).then_some(model),
            positions: positions_origin,
            retired: None,
            change: self.change,
            key: self.key,
            stamp: self.stamp.clone(),
            next: Some(Projection {
                graph,
                synopsis,
                paths,
                positions,
                stamp: Some(self.stamp),
                placement: status,
            }),
        })
    }
}
