//! Keep the line being written in view when the manuscript's viewport shrinks under it, as when a
//! build result's options open below the work.
use super::*;
use crate::attention::EditingTarget;
use xilem::masonry::{
    accesskit::{Action as NativeAction, ActionRequest, TreeId},
    kurbo::{Affine, Rect},
};

/// The manuscript viewport last seen: its widget and height.
#[derive(Default)]
pub(super) struct InView {
    viewport: Option<(WidgetId, f64)>,
}
impl InView {
    pub fn synchronize(&mut self, root: &mut RenderRoot, editing: Option<&EditingTarget>) {
        if root.needs_rewrite_passes() {
            return;
        }
        let Some(portal) = super::reading::manuscript(root, self.viewport.map(|(id, _)| id))
            .filter(|portal| !portal.ctx().is_stashed())
        else {
            self.viewport = None;
            return;
        };
        let id = portal.id();
        let bounds = window_bounds(portal.ctx().window_transform(), portal.ctx().border_box());
        let previous = self
            .viewport
            .replace((id, bounds.height()))
            .filter(|(known, _)| *known == id)
            .map(|(_, height)| height);
        let Some(field) =
            editing.and_then(|target| super::manuscript::find(root.get_layer_root(0), target))
        else {
            return;
        };
        let line = root.get_widget(field).map(|widget| {
            window_bounds(widget.ctx().window_transform(), widget.ctx().border_box())
        });
        if line.is_some_and(|line| shrank_away(previous, bounds, line)) {
            root.handle_access_event(ActionRequest {
                action: NativeAction::ScrollIntoView,
                target_tree: TreeId::ROOT,
                target_node: field.to_raw().into(),
                data: None,
            });
        }
    }
}

fn window_bounds(transform: Affine, rect: Rect) -> Rect {
    transform.transform_rect_bbox(rect)
}

/// Whether a viewport that was `previous` tall shrank to `viewport` and left `line` out of it.
pub(super) fn shrank_away(previous: Option<f64>, viewport: Rect, line: Rect) -> bool {
    previous.is_some_and(|height| viewport.height() < height - 0.5)
        && (line.y0 < viewport.y0 || line.y1 > viewport.y1)
}

#[cfg(test)]
mod tests;
