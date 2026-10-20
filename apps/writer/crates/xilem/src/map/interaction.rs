//! Gestures carry their origin; cancellation restores it before any layout commit.
use super::*;
use crate::app::picker;
use input::Input;
use recite_writer_session::map::placement::card_key;
use xilem::masonry::kurbo::Vec2;

#[derive(Clone)]
pub(super) enum Source {
    Viewport,
    Grip(String),
    Card(String),
}
impl Writer {
    pub(super) fn map_input(&mut self, source: Source, action: Input) {
        if matches!(action, Input::Cancel) {
            self.cancel_map_gesture();
            return;
        }
        if !self.map.workspace.has_map() || self.phase != crate::Phase::Writing {
            return;
        }
        if matches!(source, Source::Grip(_))
            && (self.navigation.busy() || self.work.busy())
            && !matches!(action, Input::Cancel)
        {
            self.cancel_map_gesture();
            return;
        }
        let current = self
            .map
            .scene(self)
            .is_some_and(|scene| scene.projection.is_current(self.session.model().document()));
        if !current {
            return;
        }
        match action {
            Input::Begin(point) => self.begin_map_gesture(source, point),
            Input::Move(point) => self.move_map_gesture(point),
            Input::Finish => self.finish_map_gesture(),
            Input::OverviewClick(point) => self.select_overview_beat(point),
            Input::Cancel => self.cancel_map_gesture(),
            Input::Direction(direction) => self.map_direction(source, direction),
            Input::Open => {
                if let Source::Card(id) = source {
                    self.select_map_beat(id);
                }
                self.open_map_beat();
            }
            Input::Search => self.pending_search = Some(picker::Search::GoTo),
            Input::Focus => {
                if let Source::Card(id) = source {
                    self.select_map_beat(id);
                }
            }
            Input::Focused(token) => {
                if let Some(scene) = self.map_scene_mut()
                    && scene
                        .focus
                        .as_ref()
                        .is_some_and(|focus| focus.token == token)
                {
                    scene.focus = None;
                }
            }
            Input::Hover(hovered) => {
                if let Source::Card(id) = source
                    && let Some(scene) = self.map_scene_mut()
                {
                    if hovered {
                        scene.hovered = Some(id);
                    } else if scene.hovered.as_ref() == Some(&id) {
                        scene.hovered = None;
                    }
                }
            }
            Input::Pan(delta) => {
                self.cancel_map_gesture();
                let scale = self.ui_scale();
                if let Some(scene) = self.map_scene_mut() {
                    scene.camera.x += delta.x as f32 / scale;
                    scene.camera.y += delta.y as f32 / scale;
                    scene.camera.framing = Framing::Free;
                }
            }
            Input::Zoom { factor, anchor } => {
                self.cancel_map_gesture();
                let scale = self.ui_scale();
                let pointer = self.config.writer.zoom_to_pointer;
                if let Some(scene) = self.map_scene_mut() {
                    let anchor = if pointer {
                        (anchor.x as f32 / scale, anchor.y as f32 / scale)
                    } else {
                        (scene.viewport.0 / 2., scene.viewport.1 / 2.)
                    };
                    scene.camera.zoom_at(scene.camera.zoom * factor, anchor);
                }
            }
        }
    }
    fn begin_map_gesture(&mut self, source: Source, point: Point) {
        let busy = self.navigation.busy() || self.work.busy();
        let Some(scene) = self.map_scene_mut() else {
            return;
        };
        scene.gesture = match source {
            Source::Viewport => Gesture::Panning {
                start: point,
                camera: scene.camera,
            },
            Source::Grip(beat) if !busy => {
                let Some(index) = scene.projection.graph.ids.get(&beat) else {
                    return;
                };
                let node = scene.projection.graph.nodes[*index].clone();
                Gesture::Moving {
                    start: point,
                    beat,
                    original: node.clone(),
                    current: node,
                    mounted: scene.visible_indices().collect(),
                }
            }
            _ => Gesture::Idle,
        };
    }
    fn move_map_gesture(&mut self, point: Point) {
        let scale = self.ui_scale();
        let Some(scene) = self.map_scene_mut() else {
            return;
        };
        match &mut scene.gesture {
            Gesture::Panning { start, camera, .. } => {
                let delta = point - *start;
                scene.camera.x = camera.x + delta.x as f32 / scale;
                scene.camera.y = camera.y + delta.y as f32 / scale;
                scene.camera.framing = Framing::Free;
            }
            Gesture::Moving {
                start,
                original,
                current,
                ..
            } => {
                let delta = point - *start;
                current.x =
                    (original.x + delta.x as f32 / (scale * scene.camera.zoom)).clamp(8., 100_000.);
                current.y = (original.y + delta.y as f32 / (scale * scene.camera.zoom))
                    .clamp(12., 100_000.);
            }
            Gesture::Idle => {}
        }
    }
    fn finish_map_gesture(&mut self) {
        let Some(scene) = self.map_scene_mut() else {
            return;
        };
        if let Gesture::Moving { beat, current, .. } = std::mem::take(&mut scene.gesture) {
            let Some(index) = scene.projection.graph.ids.get(&beat) else {
                return;
            };
            let card = card_key(&scene.projection.graph.blocks[*index]);
            scene.placement = Some(placement::Change::Move {
                beat,
                card,
                node: current,
            });
            self.refresh_map_layout();
        }
    }
    pub(crate) fn cancel_map_gesture(&mut self) {
        if let Some(scene) = self.map_scene_mut()
            && let Gesture::Panning { camera, .. } = std::mem::take(&mut scene.gesture)
        {
            scene.camera = camera;
        }
    }
    fn map_direction(&mut self, source: Source, direction: Vec2) {
        let Some(scene) = self.map_scene_mut() else {
            return;
        };
        match source {
            Source::Grip(beat) => {
                let Some(index) = scene.projection.graph.ids.get(&beat) else {
                    return;
                };
                let mut node = scene.node(*index).clone();
                node.x = (node.x + direction.x as f32 * 24.).clamp(8., 100_000.);
                node.y = (node.y + direction.y as f32 * 24.).clamp(12., 100_000.);
                let card = card_key(&scene.projection.graph.blocks[*index]);
                scene.placement = Some(placement::Change::Move { beat, card, node });
                self.refresh_map_layout();
            }
            Source::Viewport | Source::Card(_) => {
                let index = scene
                    .selected
                    .as_ref()
                    .and_then(|id| scene.projection.graph.ids.get(id))
                    .copied()
                    .unwrap_or(0);
                if let Some(next) = navigation::nearest(
                    &scene.projection.graph.nodes,
                    index,
                    (direction.x as f32, direction.y as f32),
                ) {
                    let beat = scene.projection.graph.blocks[next].id.clone();
                    self.focus_map_beat(beat);
                }
            }
        }
    }
}
