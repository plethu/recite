//! Preserve a visible, retained native passage across an explicit overlapping window shift.
use super::*;
use crate::{attention::PassageField, view::ReadingViewport};
use xilem::masonry::{
    accesskit::{Action as NativeAction, ActionRequest, TreeId},
    kurbo::{Point, Rect},
    widgets::{Flex, Portal},
};

struct Row {
    id: WidgetId,
    bounds: Rect,
}
struct Snapshot {
    scope: ReadingViewport,
    portal: WidgetId,
    bounds: Rect,
    rows: Vec<Row>,
    position: Point,
}
#[derive(Default)]
pub(super) struct Position {
    previous: Option<Snapshot>,
}
impl Position {
    pub fn synchronize(&self, root: &mut RenderRoot) {
        let Some(previous) = &self.previous else {
            return;
        };
        if root
            .get_widget(previous.portal)
            .and_then(|widget| widget.downcast::<Portal<Flex>>())
            .is_none_or(|portal| {
                !portal
                    .get_prop::<ReadingViewport>()
                    .shifted_from(&previous.scope)
            })
        {
            return;
        }
        let Some(current) = snapshot(root) else {
            return;
        };
        if current.portal != previous.portal || !current.scope.shifted_from(&previous.scope) {
            return;
        }
        let anchor = previous
            .rows
            .iter()
            .filter(|row| row.bounds.y1 > previous.bounds.y0 && row.bounds.y0 < previous.bounds.y1)
            .filter_map(|old| {
                current
                    .rows
                    .iter()
                    .find(|row| row.id == old.id)
                    .map(|new| (old, new))
            })
            .min_by(|(a, _), (b, _)| {
                (a.bounds.y0 - previous.bounds.y0)
                    .abs()
                    .total_cmp(&(b.bounds.y0 - previous.bounds.y0).abs())
            });
        if let Some((old, new)) = anchor {
            let delta = (new.bounds.y0 - current.bounds.y0) - (old.bounds.y0 - previous.bounds.y0);
            root.edit_widget(current.portal, |mut widget| {
                let mut portal = widget.downcast::<Portal<Flex>>();
                Portal::set_viewport_pos(
                    &mut portal,
                    Point::new(current.position.x, current.position.y + delta),
                );
            });
        } else if let Some(row) = current.rows.first() {
            root.handle_access_event(ActionRequest {
                action: NativeAction::ScrollIntoView,
                target_tree: TreeId::ROOT,
                target_node: row.id.to_raw().into(),
                data: None,
            });
        }
    }
    pub fn capture(&mut self, root: &RenderRoot) {
        self.previous = snapshot(root);
    }
}
fn snapshot(root: &RenderRoot) -> Option<Snapshot> {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        if let Some(portal) = widget.downcast::<Portal<Flex>>()
            && portal.get_prop::<ReadingViewport>().is_manuscript()
        {
            let bounds = widget
                .ctx()
                .window_transform()
                .transform_rect_bbox(widget.ctx().content_box());
            let mut pending = widget.children();
            let mut rows = Vec::new();
            while let Some(widget) = pending.pop() {
                if widget.ctx().is_stashed() {
                    continue;
                }
                if let Some(input) = widget.downcast::<TextInput>()
                    && input.get_prop::<PassageField>().0.is_some()
                {
                    rows.push(Row {
                        id: widget.id(),
                        bounds: widget
                            .ctx()
                            .window_transform()
                            .transform_rect_bbox(widget.ctx().border_box()),
                    });
                } else {
                    pending.extend(widget.children());
                }
            }
            rows.sort_by(|a, b| a.bounds.y0.total_cmp(&b.bounds.y0));
            return Some(Snapshot {
                scope: portal.get_prop::<ReadingViewport>().clone(),
                portal: widget.id(),
                bounds,
                rows,
                position: portal.get_viewport_pos(),
            });
        }
        pending.extend(widget.children());
    }
    None
}
