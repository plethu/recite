//! Structural overview paints geometry without mounting thousands of tiny text controls.
use super::*;
use xilem::masonry::imaging::record;

pub(super) fn draw(state: &Writer, scene: &Scene, recording: &mut record::Scene) {
    let scale = f64::from(state.ui_scale());
    let mut painter = Painter::new(recording);
    for index in scene.visible_indices() {
        let selected = scene.selected.as_deref() == Some(&scene.projection.graph.blocks[index].id);
        let bounds = scene.overview_bounds(index, scale);
        painter
            .fill(
                bounds,
                if selected {
                    state.palette.selection
                } else {
                    state.palette.surface
                },
            )
            .draw();
        painter
            .stroke(
                bounds,
                &Stroke::new(if selected { 2. } else { 1. }),
                if selected {
                    state.palette.accent
                } else {
                    state.palette.muted
                },
            )
            .draw();
    }
}
