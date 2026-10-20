//! Overview activation changes presentation and selection without editing source.
use super::*;
use recite_writer_session::map::layout::HEIGHT;
use xilem::masonry::kurbo::Rect;

impl Scene {
    pub(super) fn overview_bounds(&self, index: usize, scale: f64) -> Rect {
        let node = self.node(index);
        let zoom = f64::from(self.camera.zoom) * scale;
        Rect::from_origin_size(
            (
                f64::from(node.x * self.camera.zoom + self.camera.x) * scale,
                f64::from(node.y * self.camera.zoom + self.camera.y) * scale,
            ),
            (
                (f64::from(node.width) * zoom).max(2.),
                (f64::from(HEIGHT) * zoom).max(2.),
            ),
        )
    }
}
impl Writer {
    pub(super) fn select_overview_beat(&mut self, point: Point) {
        let scale = f64::from(self.ui_scale());
        let Some(scene) = self
            .map
            .scene(self)
            .filter(|scene| scene.detail() == Detail::Overview)
        else {
            return;
        };
        let selected = scene
            .node_indices()
            .filter_map(|index| {
                let bounds = scene.overview_bounds(index, scale);
                bounds
                    .inflate(4., 4.)
                    .contains(point)
                    .then(|| (index, (point - bounds.center()).hypot2()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| scene.projection.graph.blocks[index].id.clone());
        if let Some(beat) = selected {
            self.focus_map_beat(beat);
        }
    }
}
