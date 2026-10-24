//! Native map presentation owns camera and selection; Workbench owns authored edits.
mod actions;
mod connections;
mod input;
mod interaction;
mod overview;
mod placed;
mod placement;
mod preparation;
mod publication;
mod scene;
mod view;
use crate::{Writer, navigation::Target};
pub(super) use preparation::{Preparation, Prepared, Purpose};
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
pub(super) use view::panel;
use xilem::masonry::kurbo::{BezPath, Point};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Workspace {
    #[default]
    Script,
    Map,
    Split,
}
impl Workspace {
    pub fn configured(view: recite_config::WriterView, split: bool) -> Self {
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
enum Detail {
    Dialogue,
    Overview,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tools {
    #[default]
    Closed,
    Actions,
    Help,
}
pub(super) struct MapSession {
    pub workspace: Workspace,
    scenes: BTreeMap<DocumentKey, Scene>,
    store: Option<Result<recite_config::UserStateFile, String>>,
    refresh: Option<DocumentKey>,
}
pub(super) struct Projection {
    graph: Arc<Topology>,
    synopsis: Vec<synopsis::Synopsis>,
    paths: Vec<Option<(BezPath, BezPath)>>,
    positions: Arc<Positions>,
    stamp: Option<recite_writer_model::AnalysisStamp>,
    placement: placement::Status,
}
impl Projection {
    fn is_current(&self, document: &recite_writer_model::Document) -> bool {
        self.stamp
            .as_ref()
            .is_some_and(|stamp| stamp.is_current(document))
    }
}
struct Scene {
    projection: Projection,
    camera: Camera,
    viewport: (f32, f32),
    selected: Option<String>,
    scope: Option<scope::Scope>,
    whole: bool,
    connections: connections::Connections,
    tools: Tools,
    search: String,
    gesture: Gesture,
    placement: Option<placement::Change>,
    focus: Option<FocusIntent>,
    hovered: Option<String>,
}
struct FocusIntent {
    beat: String,
    token: input::FocusToken,
}
#[derive(Default)]
enum Gesture {
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
impl MapSession {
    pub fn new() -> Self {
        Self {
            workspace: Workspace::Script,
            scenes: BTreeMap::new(),
            store: None,
            refresh: None,
        }
    }
    pub fn personal_storage(&mut self) {
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
    pub fn cancel_gestures(&mut self) {
        for scene in self.scenes.values_mut() {
            if let Gesture::Panning { camera, .. } = std::mem::take(&mut scene.gesture) {
                scene.camera = camera;
            }
        }
    }
    fn scene(&self, writer: &Writer) -> Option<&Scene> {
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

#[cfg(test)]
mod tests;
