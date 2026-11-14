//! Reveal only the result of a native Tab move, after Masonry handles the key.
//!
//! Masonry offers every scrollable Portal as a Tab stop but gives it no accessible name, and its
//! focus draws nothing. Writer's panes keep keyboard scrolling without being stops: page and
//! arrow keys from any control inside bubble to the pane, and its scrollbars stay focusable. So a
//! Tab move that lands on a pane continues to the next stop in the same direction.
use super::*;
use xilem::masonry::{accesskit::Role, core::WidgetRef};

#[derive(Default)]
pub(super) enum Reveal {
    #[default]
    Idle,
    Pending {
        from: Option<WidgetId>,
        backward: bool,
    },
}
impl Reveal {
    pub fn request(&mut self, from: Option<WidgetId>, backward: bool) {
        *self = Self::Pending { from, backward };
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        let Self::Pending { from, backward } = std::mem::take(self) else {
            return;
        };
        past_panes(root, backward);
        if root
            .focused_widget()
            .is_some_and(|current| Some(current) != from)
        {
            panes::reveal_focused(root);
        }
    }
}

/// Move focus off an unnamed scroll pane to the next Tab stop in the same direction.
pub(crate) fn past_panes(root: &mut RenderRoot, backward: bool) {
    let Some(focused) = root.focused_widget() else {
        return;
    };
    let stops = stops(root.get_layer_root(0));
    let Some(index) = stops.iter().position(|(id, _)| *id == focused) else {
        return;
    };
    if !stops[index].1 {
        return;
    }
    let count = stops.len();
    let next = (1..count)
        .map(|step| {
            if backward {
                (index + count - step) % count
            } else {
                (index + step) % count
            }
        })
        .find(|candidate| !stops[*candidate].1);
    if let Some(next) = next {
        root.focus_on(Some(stops[next].0));
    }
}

/// Focusable widgets in Masonry's Tab order (preorder), each marked when it is a scroll pane.
fn stops(root: WidgetRef<'_, dyn Widget>) -> Vec<(WidgetId, bool)> {
    let mut stops = Vec::new();
    let mut pending = vec![root];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        if widget.ctx().accepts_focus() && !widget.ctx().is_disabled() {
            stops.push((widget.id(), is_pane(&widget)));
        }
        pending.extend(widget.children().into_iter().rev());
    }
    stops
}

fn is_pane(widget: &WidgetRef<'_, dyn Widget>) -> bool {
    widget.accessibility_role() == Role::ScrollView
}

#[cfg(test)]
mod tests;
