//! Keep a newly focused control's focus ring inside the slip or dialog scroll region that holds
//! it, as when a slip opens with its first control at the visible edge of its scrolling content.
//! Elsewhere focus alone never scrolls.
use super::*;
use crate::app::keyboard::ModalSurface;
use crate::presentation::tokens::size;
use xilem::masonry::{
    core::WidgetRef,
    kurbo::{Rect, Vec2},
    widgets::{Flex, Portal},
};

/// How far a focus ring reaches past its control: the outline's gap, its 2px stroke and a pixel.
const RING: f64 = size::FOCUS_OUTLINE + 3.;

/// The control whose ring still has to be brought into view, once layout has settled.
#[derive(Default)]
pub(super) struct RingInView {
    seen: Option<WidgetId>,
    pending: Option<WidgetId>,
}
impl RingInView {
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        let focused = root.focused_widget();
        if focused != self.seen {
            self.seen = focused;
            self.pending = focused;
        }
        if root.needs_rewrite_passes() {
            return;
        }
        let Some(target) = self.pending.take() else {
            return;
        };
        let Some((portal, pan)) = pan_for(root, target) else {
            return;
        };
        root.edit_widget(portal, |mut widget| {
            Portal::pan_viewport_by(&mut widget.downcast::<Portal<Flex>>(), Vec2::new(0., pan));
        });
    }
}

/// The innermost scroll region inside a slip or dialog holding `target`, and how far it must pan to
/// show `target`'s ring.
fn pan_for(root: &RenderRoot, target: WidgetId) -> Option<(WidgetId, f64)> {
    let mut portals = Vec::new();
    let found = portals_above(root.get_layer_root(0), target, false, &mut portals);
    let portal = root.get_widget(*portals.last().filter(|_| found)?)?;
    let bounds = |widget: &WidgetRef<'_, dyn Widget>| {
        widget
            .ctx()
            .window_transform()
            .transform_rect_bbox(widget.ctx().border_box())
    };
    let focused = bounds(&root.get_widget(target)?);
    let pan = ring_pan(bounds(&portal), focused);
    (pan != 0.).then_some((portal.id(), pan))
}

/// Collects the scroll regions inside a modal surface on the way from `widget` to `target`, true
/// once it is found.
fn portals_above(
    widget: WidgetRef<'_, dyn Widget>,
    target: WidgetId,
    modal: bool,
    portals: &mut Vec<WidgetId>,
) -> bool {
    if widget.id() == target {
        return true;
    }
    if widget.ctx().is_stashed() {
        return false;
    }
    let modal = modal || widget.get_prop::<ModalSurface>().0;
    let portal = modal && widget.downcast::<Portal<Flex>>().is_some();
    if portal {
        portals.push(widget.id());
    }
    if widget
        .children()
        .into_iter()
        .any(|child| portals_above(child, target, modal, portals))
    {
        return true;
    }
    if portal {
        portals.pop();
    }
    false
}

/// How far a viewport showing `viewport` must pan down (or up, when negative) to show `focused`
/// with its ring, preferring the control's top where both edges cannot show.
pub(super) fn ring_pan(viewport: Rect, focused: Rect) -> f64 {
    let ringed = focused.inflate(0., RING);
    if ringed.y0 < viewport.y0 {
        ringed.y0 - viewport.y0
    } else if ringed.y1 > viewport.y1 {
        (ringed.y1 - viewport.y1).min(ringed.y0 - viewport.y0)
    } else {
        0.
    }
}

#[cfg(test)]
mod tests;
