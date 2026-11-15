//! The galley keeps its place. A newly chosen beat's heading is brought to the top of the sheet,
//! and the running head shows once the scene title has scrolled out of the sheet.
use super::*;
use crate::presentation::running::RunningHead;
use crate::presentation::space;
use crate::view::{ReadingViewport, SelectedBeat};
use xilem::masonry::{
    core::WidgetRef,
    kurbo::Point,
    widgets::{Flex, Portal},
};

/// Scrolled further than this, the scene title has left the sheet and the running head shows.
const TITLE: f64 = 72.;
/// A chosen beat's heading lands a group's space below the running head.
const LEAD: f64 = space::GROUPS;
/// The running head's height at 100%, until a shown head has been measured.
const HEAD: f64 = 33.;

pub(crate) struct Galley {
    /// The reading the galley last brought into view.
    seen: Option<ReadingViewport>,
    /// A newly chosen beat waits for layout before its heading is measured.
    pending: bool,
    /// The running head's height as last laid out, which grows with the interface scale.
    head: f64,
    /// Where the galley last scrolled to, so a correction never undoes the writer's own scroll.
    revealed: Option<f64>,
}
impl Default for Galley {
    fn default() -> Self {
        Self {
            seen: None,
            pending: false,
            head: HEAD,
            revealed: None,
        }
    }
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
            self.revealed = reveal_selected(root, portal, self.head + LEAD);
        }
        let Some(height) = show_running_head(root, portal) else {
            return;
        };
        // A head first measured at another height (another interface scale) would cover the
        // heading just brought up; bring it clear again unless the writer has scrolled since.
        if (height - self.head).abs() > 0.5 {
            self.head = height;
            self.pending = self.revealed.is_some() && self.revealed == viewport(root, portal);
        }
    }
}

/// Scroll so the selected beat's heading sits near the top of the sheet; the first beat keeps the
/// scene title in view.
/// The heading lands `lead` below the sheet's top edge. Returns where the galley scrolled to.
fn reveal_selected(root: &mut RenderRoot, portal: WidgetId, lead: f64) -> Option<f64> {
    let target = root
        .get_widget(portal)
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
        .and_then(|portal| heading_offset(&portal))?;
    let y = if target < TITLE + lead {
        0.
    } else {
        target - lead
    };
    root.edit_widget(portal, |mut widget| {
        let mut portal = widget.downcast::<Portal<Flex>>();
        Portal::set_viewport_pos(&mut portal, Point::new(0., y));
    });
    viewport(root, portal)
}

fn viewport(root: &RenderRoot, portal: WidgetId) -> Option<f64> {
    root.get_widget(portal)
        .and_then(|widget| widget.downcast::<Portal<Flex>>())
        .map(|portal| portal.get_viewport_pos().y)
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

/// Show the running head over the sheet once its manuscript has scrolled past the title. Returns
/// the height of a head that was already shown and laid out.
fn show_running_head(root: &mut RenderRoot, portal: WidgetId) -> Option<f64> {
    let scrolled = viewport(root, portal)? > TITLE;
    let mut pending = vec![root.get_layer_root(0)];
    let mut heads = Vec::new();
    let mut height = None;
    while let Some(widget) = pending.pop() {
        if let Some(head) = widget.downcast::<RunningHead>() {
            if head.inner().is_shown() != scrolled {
                heads.push(widget.id());
            } else if scrolled {
                height = Some(widget.ctx().border_box().height());
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
    height
}

#[cfg(test)]
mod tests;
