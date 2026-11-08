//! Native map presentation owns camera and selection; Workbench owns authored edits.
pub mod actions;
pub mod connections;
pub mod input;
pub mod interaction;
mod overview;
mod placement;
pub mod preparation;
mod publication;
pub mod scene;
pub use crate::{Writer, navigation::Target};
use kurbo::{BezPath, Point};
pub use preparation::{Preparation, Prepared, Purpose};
use recite_core::DocumentKey;
use recite_writer_session::map::synopsis;
use recite_writer_session::map::{
    camera::{Camera, Framing},
    layout::Node,
    navigation,
    placement::Positions,
    scope,
    topology::Topology,
};
use std::collections::btree_map;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Workspace {
    #[default]
    Script,
    Map,
    Split,
}
impl Workspace {
    pub(crate) fn configured(view: recite_config::WriterView, split: bool) -> Self {
        if split {
            Self::Split
        } else if view == recite_config::WriterView::Map {
            Self::Map
        } else {
            Self::Script
        }
    }
    pub fn has_map(self) -> bool {
        self != Self::Script
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    Dialogue,
    Overview,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Tools {
    #[default]
    Closed,
    Actions,
    Help,
}
pub struct MapSession {
    pub workspace: Workspace,
    scenes: BTreeMap<DocumentKey, Scene>,
    pub store: Option<Result<recite_config::UserStateFile, String>>,
    refresh: Option<DocumentKey>,
}
pub struct Projection {
    pub graph: Arc<Topology>,
    pub synopsis: Vec<synopsis::Synopsis>,
    pub paths: Vec<Option<(BezPath, BezPath)>>,
    pub positions: Arc<Positions>,
    stamp: Option<recite_writer_model::AnalysisStamp>,
    pub placement: placement::Status,
}
impl Projection {
    pub fn is_current(&self, document: &recite_writer_model::Document) -> bool {
        self.stamp
            .as_ref()
            .is_some_and(|stamp| stamp.is_current(document))
    }
}
pub struct Scene {
    pub projection: Projection,
    pub camera: Camera,
    pub viewport: (f32, f32),
    pub selected: Option<String>,
    pub scope: Option<scope::Scope>,
    pub whole: bool,
    pub connections: connections::Connections,
    pub tools: Tools,
    pub search: String,
    pub gesture: Gesture,
    placement: Option<placement::Change>,
    pub focus: Option<FocusIntent>,
    pub hovered: Option<String>,
}
pub struct FocusIntent {
    pub beat: String,
    pub token: input::FocusToken,
}
#[derive(Default)]
pub enum Gesture {
    #[default]
    Idle,
    Panning {
        start: Point,
        camera: Camera,
    },
    Moving {
        start: Point,
        beat: String,
        original: Node,
        current: Node,
        mounted: Vec<usize>,
    },
}
impl Gesture {
    fn mounted(&self) -> Option<&[usize]> {
        match self {
            Self::Idle | Self::Panning { .. } => None,
            Self::Moving { mounted, .. } => Some(mounted),
        }
    }
}
impl Default for MapSession {
    fn default() -> Self {
        Self::new()
    }
}
impl MapSession {
    pub fn new() -> Self {
        Self {
            workspace: Workspace::Script,
            scenes: BTreeMap::new(),
            store: None,
            refresh: None,
        }
    }
    pub(crate) fn personal_storage(&mut self) {
        self.store = Some(
            recite_config::UserConfigStore::discover()
                .map_err(|error| error.to_string())
                .and_then(|store| {
                    store
                        .state_file("writer-layouts.json")
                        .map_err(|error| error.to_string())
                }),
        );
    }
    pub(crate) fn cancel_gestures(&mut self) {
        for scene in self.scenes.values_mut() {
            if let Gesture::Panning { camera, .. } = std::mem::take(&mut scene.gesture) {
                scene.camera = camera;
            }
        }
    }
    pub fn scene(&self, writer: &Writer) -> Option<&Scene> {
        self.scenes.get(writer.session.model().document().key())
    }
    fn install(
        &mut self,
        key: DocumentKey,
        projection: Projection,
        selected: Option<String>,
        workspace: Workspace,
    ) -> Option<Projection> {
        self.workspace = workspace;
        match self.scenes.entry(key) {
            btree_map::Entry::Occupied(mut entry) => {
                let scene = entry.get_mut();
                if !matches!(scene.gesture, Gesture::Panning { .. }) {
                    scene.gesture = Gesture::Idle;
                }
                let camera = scene.camera;
                scene.placement = None;
                scene.focus = None;
                let previous = std::mem::replace(&mut scene.projection, projection);
                scene.selected = scene.selected.take().or(selected);
                scene.reconcile_selection();
                scene.camera = camera;
                Some(previous)
            }
            btree_map::Entry::Vacant(entry) => {
                let mut scene = Scene {
                    projection,
                    camera: Camera::default(),
                    viewport: (900., 500.),
                    selected: None,
                    scope: None,
                    whole: false,
                    connections: Default::default(),
                    tools: Tools::Closed,
                    search: String::new(),
                    gesture: Gesture::Idle,
                    placement: None,
                    focus: None,
                    hovered: None,
                };
                scene.selected = selected;
                scene.reconcile_selection();
                scene.frame();
                entry.insert(scene);
                None
            }
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests;
