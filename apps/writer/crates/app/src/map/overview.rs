//! Overview activation changes presentation and selection without editing source.
use super::*;
use kurbo::Rect;
use recite_writer_session::map::layout::HEIGHT;

impl Scene {
    pub fn overview_bounds(&self, index: usize) -> Rect {
        let node = self.node(index);
        let zoom = f64::from(self.camera.zoom);
        Rect::from_origin_size(
            (
                f64::from(node.x * self.camera.zoom + self.camera.x),
                f64::from(node.y * self.camera.zoom + self.camera.y),
            ),
            (
                (f64::from(node.width) * zoom).max(2.),
                (f64::from(HEIGHT) * zoom).max(2.),
            ),
        )
    }
}
impl Writer {
    pub fn select_overview_beat(&mut self, point: Point) {
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
                let bounds = scene.overview_bounds(index);
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
