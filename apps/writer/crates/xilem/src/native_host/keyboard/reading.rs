//! Preserve a visible, retained native passage across an explicit overlapping window shift.
use super::*;
use crate::{attention::PassageField, view::ReadingViewport};
use xilem::masonry::{
    accesskit::{Action as NativeAction, ActionRequest, TreeId},
    core::WidgetRef,
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
    /// The manuscript viewport, revalidated before reuse. Synchronization runs around every
    /// native event, and Map workspaces keep it stashed rather than unmounted.
    portal: Option<WidgetId>,
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
        let Some(current) = manuscript(root, self.portal).and_then(snapshot) else {
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
        let portal = manuscript(root, self.portal);
        self.portal = portal.map(|portal| portal.id());
        self.previous = portal.and_then(snapshot);
    }
}
/// Finds the manuscript viewport, reusing a known one while it remains that viewport.
///
/// The search includes stashed panes: a Map workspace opened directly keeps Script stashed,
/// and its viewport must still be found once. [`snapshot`] refuses a stashed viewport.
fn manuscript(root: &RenderRoot, known: Option<WidgetId>) -> Option<WidgetRef<'_, Portal<Flex>>> {
    let is_manuscript =
        |portal: &WidgetRef<'_, Portal<Flex>>| portal.get_prop::<ReadingViewport>().is_manuscript();
    if let Some(portal) = known
        .and_then(|id| root.get_widget(id))
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
        .filter(is_manuscript)
    {
        return Some(portal);
    }
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        #[cfg(test)]
        tests::WALKED.with(|walked| walked.set(walked.get() + 1));
        if let Some(portal) = widget.downcast::<Portal<Flex>>().filter(is_manuscript) {
            return Some(portal);
        }
        pending.extend(widget.children());
    }
    None
}
fn snapshot(portal: WidgetRef<'_, Portal<Flex>>) -> Option<Snapshot> {
    if portal.ctx().is_stashed() {
        return None;
    }
    let bounds = portal
        .ctx()
        .window_transform()
        .transform_rect_bbox(portal.ctx().content_box());
    let mut pending = portal.as_dyn().children();
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
    Some(Snapshot {
        scope: portal.get_prop::<ReadingViewport>().clone(),
        portal: portal.id(),
        bounds,
        rows,
        position: portal.get_viewport_pos(),
    })
}

#[cfg(test)]
mod tests;
