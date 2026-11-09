//! Selection, visibility and camera invariants for one accepted scene.
use super::*;
use recite_writer_session::map::layout;
use recite_writer_session::map::topology;

impl Scene {
    pub fn detail(&self) -> Detail {
        if self.camera.zoom < recite_writer_session::map::DIALOGUE_ZOOM {
            Detail::Overview
        } else {
            Detail::Dialogue
        }
    }
    pub fn moved_beat(&self) -> Option<&str> {
        match (&self.gesture, &self.placement) {
            (Gesture::Moving { beat, .. }, _) | (_, Some(placement::Change::Move { beat, .. })) => {
                Some(beat)
            }
            _ => None,
        }
    }
    pub fn visible_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.node_indices().filter(|index| {
            self.selected.as_deref() == Some(&self.projection.graph.blocks[*index].id)
                || topology::visible(
                    (
                        self.node(*index).x,
                        self.node(*index).y,
                        self.node(*index).width,
                        layout::HEIGHT,
                    ),
                    self.camera,
                    self.viewport,
                )
        })
    }
    pub fn mounted_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.gesture
            .mounted()
            .into_iter()
            .flatten()
            .copied()
            .chain(
                self.visible_indices()
                    .take(if self.gesture.mounted().is_none() {
                        usize::MAX
                    } else {
                        0
                    }),
            )
            .take(if self.detail() == Detail::Dialogue {
                usize::MAX
            } else {
                0
            })
    }
    pub(super) fn node_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.scope
            .iter()
            .flat_map(|scope| scope.node_indices.iter().copied())
            .chain(
                (0..self.projection.graph.nodes.len()).take(if self.scope.is_none() {
                    self.projection.graph.nodes.len()
                } else {
                    0
                }),
            )
    }
    pub fn edge_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.scope
            .iter()
            .flat_map(|scope| scope.edge_indices.iter().copied())
            .chain(
                (0..self.projection.graph.links.len()).take(if self.scope.is_none() {
                    self.projection.graph.links.len()
                } else {
                    0
                }),
            )
    }
    pub(super) fn select(&mut self, selected: Option<String>) {
        if self.selected == selected
            && selected
                .as_ref()
                .is_none_or(|id| self.projection.graph.ids.contains_key(id))
        {
            return;
        }
        self.selected = selected;
        self.focus = None;
        self.reconcile_selection();
        self.reveal_selected();
    }
    pub(super) fn reveal_selected(&mut self) {
        if let Some(node) = self
            .selected
            .as_ref()
            .and_then(|id| self.projection.graph.ids.get(id))
            .and_then(|index| self.projection.graph.nodes.get(*index))
        {
            navigation::reveal(&mut self.camera, node, self.viewport);
        }
    }
    pub fn reconcile_selection(&mut self) {
        let selected = self.selected.take();
        self.selected = selected
            .filter(|id| self.projection.graph.ids.contains_key(id))
            .or_else(|| {
                self.projection
                    .graph
                    .blocks
                    .iter()
                    .find(|block| block.is_default)
                    .or_else(|| self.projection.graph.blocks.first())
                    .map(|block| block.id.clone())
            });
        self.connections
            .refresh(&self.projection.graph, self.selected.as_deref());
        self.scope = (!self.whole)
            .then(|| scope::nearby(&self.projection.graph, self.selected.as_deref()))
            .flatten();
    }
    pub(super) fn frame(&mut self) {
        match self.camera.framing {
            Framing::Entry => {
                if let Some(entry) = self.projection.graph.entry {
                    self.camera.frame_entry(self.viewport, entry);
                }
            }
            Framing::Fit => self.camera.fit(self.viewport, self.bounds()),
            Framing::Free => {}
        }
    }
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        self.scope
            .as_ref()
            .map_or(self.projection.graph.bounds, |scope| scope.bounds)
    }
    pub fn node(&self, index: usize) -> &Node {
        if let Gesture::Moving { beat, current, .. } = &self.gesture
            && self.projection.graph.blocks[index].id == *beat
        {
            return current;
        }
        if let Some(placement::Change::Move { beat, node, .. }) = &self.placement
            && self.projection.graph.blocks[index].id == *beat
        {
            return node;
        }
        &self.projection.graph.nodes[index]
    }
}
