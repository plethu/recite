//! Reveal only the result of a native Tab move, after Masonry handles the key.
//!
//! Writer's scroll panes are named and leave the Tab order themselves (`presentation::pane`).
//!
//! A mounted modal surface (a slip or sheet marked `ModalSurface`) holds Tab: stops outside it are
//! passed over, so Tab and Shift+Tab cycle within it.
//!
//! Masonry also makes every radio a Tab stop. A radio group is one stop, its checked choice (or
//! its first, while none is checked), and arrow keys move within it (see `radio`). So a Tab move
//! that lands outside the modal surface or on another choice of a group continues in the same
//! direction.
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
        to_tab_stop(root, backward);
        if root
            .focused_widget()
            .is_some_and(|current| Some(current) != from)
        {
            panes::reveal_focused(root);
        }
    }
}

/// Move focus off a control behind a modal surface, or a radio that is not its group's stop, to
/// the next Tab stop in the same direction.
pub(crate) fn to_tab_stop(root: &mut RenderRoot, backward: bool) {
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
            let passed = (modal && !focusable.modal)
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

/// The enabled tabs of the tab list that holds `tab`, in order. Unselected tabs are not Tab
/// stops, so they are found by role, not by focus.
pub(super) fn tabs(root: WidgetRef<'_, dyn Widget>, tab: WidgetId) -> Vec<WidgetId> {
    fn collect(
        widget: WidgetRef<'_, dyn Widget>,
        list: Option<WidgetId>,
        found: &mut Vec<(WidgetId, WidgetId)>,
    ) {
        if widget.ctx().is_stashed() {
            return;
        }
        let role = widget.accessibility_role();
        let list = if role == Role::TabList {
            Some(widget.id())
        } else {
            list
        };
        if role == Role::Tab
            && !widget.ctx().is_disabled()
            && let Some(list) = list
        {
            found.push((list, widget.id()));
        }
        for child in widget.children() {
            collect(child, list, found);
        }
    }
    let mut found = Vec::new();
    collect(root, None, &mut found);
    let Some(list) = found
        .iter()
        .find(|(_, id)| *id == tab)
        .map(|(list, _)| *list)
    else {
        return Vec::new();
    };
    found
        .into_iter()
        .filter(|(owner, _)| *owner == list)
        .map(|(_, id)| id)
        .collect()
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

#[cfg(test)]
mod tests;
