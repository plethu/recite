//! Reveal only the result of a native Tab move, after Masonry handles the key.
//!
//! Masonry offers every scrollable Portal as a Tab stop but gives it no accessible name, and its
//! focus draws nothing. Writer's panes keep keyboard scrolling without being stops: page and
//! arrow keys from any control inside bubble to the pane, and its scrollbars stay focusable.
//!
//! A mounted modal surface (a slip or sheet marked `ModalSurface`) holds Tab: stops outside it are
//! passed over, so Tab and Shift+Tab cycle within it.
//!
//! Masonry also makes every radio a Tab stop. A radio group is one stop, its checked choice (or
//! its first, while none is checked), and arrow keys move within it (see `radio`). So a Tab move
//! that lands on a pane or on another choice of a group continues in the same direction.
use super::*;
use crate::app::keyboard::ModalSurface;
use crate::presentation::GroupStop;
use xilem::masonry::{accesskit::Role, core::WidgetRef, widgets::SizedBox};

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

/// Move focus off an unnamed scroll pane, or a radio that is not its group's stop, to the next
/// Tab stop in the same direction.
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

/// One focusable widget in Masonry's Tab order.
struct Focusable {
    id: WidgetId,
    pane: bool,
    /// Inside a modal surface.
    modal: bool,
    /// The radio group it belongs to, and whether it is checked.
    group: Option<(WidgetId, bool)>,
}

/// Focusable widgets in Masonry's Tab order (preorder), each marked when Tab passes over it.
fn stops(root: WidgetRef<'_, dyn Widget>) -> Vec<(WidgetId, bool)> {
    let mut found = Vec::new();
    walk(root, None, false, &mut found);
    let modal = found.iter().any(|focusable| focusable.modal);
    let stop = |group: WidgetId| {
        let mut members = found
            .iter()
            .filter(|other| other.group.is_some_and(|(id, _)| id == group));
        let first = members.clone().next().map(|member| member.id);
        members
            .find(|member| member.group.is_some_and(|(_, checked)| checked))
            .map(|member| member.id)
            .or(first)
    };
    found
        .iter()
        .map(|focusable| {
            let passed = focusable.pane
                || (modal && !focusable.modal)
                || focusable
                    .group
                    .is_some_and(|(group, _)| stop(group) != Some(focusable.id));
            (focusable.id, passed)
        })
        .collect()
}

fn walk(
    widget: WidgetRef<'_, dyn Widget>,
    group: Option<WidgetId>,
    modal: bool,
    found: &mut Vec<Focusable>,
) {
    if widget.ctx().is_stashed() {
        return;
    }
    let modal = modal
        || widget
            .downcast::<SizedBox>()
            .is_some_and(|surface| surface.get_prop::<ModalSurface>().0);
    let role = widget.accessibility_role();
    let group = if role == Role::RadioGroup {
        Some(widget.id())
    } else {
        group
    };
    if widget.ctx().accepts_focus() && !widget.ctx().is_disabled() {
        found.push(Focusable {
            id: widget.id(),
            pane: is_pane(&widget),
            modal,
            group: (role == Role::RadioButton)
                .then_some(group)
                .flatten()
                .map(|group| (group, widget.get_prop::<GroupStop>().0)),
        });
    }
    for child in widget.children() {
        walk(child, group, modal, found);
    }
}

/// The enabled choices of the radio group that holds `radio`, in Tab order.
pub(super) fn choices(root: WidgetRef<'_, dyn Widget>, radio: WidgetId) -> Vec<WidgetId> {
    let mut found = Vec::new();
    walk(root, None, false, &mut found);
    let Some(group) = found
        .iter()
        .find(|focusable| focusable.id == radio)
        .and_then(|focusable| focusable.group)
        .map(|(group, _)| group)
    else {
        return Vec::new();
    };
    found
        .into_iter()
        .filter(|focusable| focusable.group.is_some_and(|(id, _)| id == group))
        .map(|focusable| focusable.id)
        .collect()
}

fn is_pane(widget: &WidgetRef<'_, dyn Widget>) -> bool {
    widget.accessibility_role() == Role::ScrollView
}

#[cfg(test)]
mod tests;
