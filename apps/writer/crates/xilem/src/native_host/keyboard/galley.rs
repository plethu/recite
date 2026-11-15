//! The galley keeps its place. A newly chosen beat's heading is brought to the top of the sheet,
//! and the running head shows once the scene title has scrolled out of the sheet.
use super::*;
use crate::presentation::running::RunningHead;
use crate::view::{ReadingViewport, SelectedBeat};
use xilem::masonry::{
    core::WidgetRef,
    kurbo::Point,
    widgets::{Flex, Portal},
};

/// Scrolled further than this, the scene title has left the sheet and the running head shows.
pub(crate) const TITLE: f64 = 72.;
/// A chosen beat's heading lands this far below the sheet's top edge.
const LEAD: f64 = 24.;

#[derive(Default)]
pub(super) struct Galley {
    /// The reading the galley last brought into view.
    seen: Option<ReadingViewport>,
    /// A newly chosen beat waits for layout before its heading is measured.
    pending: bool,
}
impl Galley {
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        let Some((portal, scope)) = reading::manuscript(root, None)
            .map(|portal| (portal.id(), portal.get_prop::<ReadingViewport>().clone()))
        else {
            return;
        };
        if self
            .seen
            .as_ref()
            .is_none_or(|seen| *seen != scope && !scope.shifted_from(seen))
        {
            self.pending = true;
        }
        self.seen = Some(scope);
        if self.pending && !root.needs_rewrite_passes() {
            self.pending = false;
            reveal_selected(root, portal);
        }
        show_running_head(root, portal);
    }
}

/// Scroll so the selected beat's heading sits near the top of the sheet; the first beat keeps the
/// scene title in view.
pub(crate) fn reveal_selected(root: &mut RenderRoot, portal: WidgetId) {
    let Some(target) = root
        .get_widget(portal)
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
        .and_then(|portal| heading_offset(&portal))
    else {
        return;
    };
    let y = if target < TITLE + LEAD {
        0.
    } else {
        target - LEAD
    };
    root.edit_widget(portal, |mut widget| {
        let mut portal = widget.downcast::<Portal<Flex>>();
        Portal::set_viewport_pos(&mut portal, Point::new(0., y));
    });
}

/// The selected beat heading's top in the manuscript content's coordinates.
fn heading_offset(portal: &WidgetRef<'_, Portal<Flex>>) -> Option<f64> {
    let bounds = portal
        .ctx()
        .window_transform()
        .transform_rect_bbox(portal.ctx().content_box());
    let origin = bounds.y0 - portal.get_viewport_pos().y;
    let mut pending = portal.as_dyn().children();
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        if widget
            .downcast::<Flex>()
            .is_some_and(|flex| flex.get_prop::<SelectedBeat>().0)
        {
            let top = widget
                .ctx()
                .window_transform()
                .transform_rect_bbox(widget.ctx().border_box())
                .y0;
            return Some(top - origin);
        }
        pending.extend(widget.children());
    }
    None
}

/// Show the running head over the sheet once its manuscript has scrolled past the title.
pub(crate) fn show_running_head(root: &mut RenderRoot, portal: WidgetId) {
    let Some(scrolled) = root
        .get_widget(portal)
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
        .map(|portal| portal.get_viewport_pos().y > TITLE)
    else {
        return;
    };
    let mut pending = vec![root.get_layer_root(0)];
    let mut heads = Vec::new();
    while let Some(widget) = pending.pop() {
        if let Some(head) = widget.downcast::<RunningHead>() {
            if head.inner().is_shown() != scrolled {
                heads.push(widget.id());
            }
            continue;
        }
        pending.extend(widget.children());
    }
    for head in heads {
        root.edit_widget(head, |mut widget| {
            RunningHead::set_shown(&mut widget.downcast::<RunningHead>(), scrolled);
        });
    }
}

#[cfg(test)]
mod tests;
