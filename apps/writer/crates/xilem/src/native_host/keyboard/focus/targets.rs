//! The widgets focus moves to: the picker's query or active result, a modal surface's default
//! control, the first problem wedge, and the control an opened picker hangs from.
use super::*;

pub(super) fn find(root: &RenderRoot, kind: Kind) -> Option<WidgetId> {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if role(widget) == kind {
            if kind == Kind::ActiveResult {
                return Some(widget.id());
            }
            return widget
                .downcast::<TextInput>()
                .map(|input| input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}
/// The enabled, mounted control marked to take focus when its modal surface opens.
pub(super) fn default_focus(root: &RenderRoot) -> Option<WidgetId> {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if widget
            .downcast::<Button>()
            .is_some_and(|button| button.get_prop::<DefaultFocus>().0)
        {
            return Some(widget.id());
        }
        if let Some(input) = widget.downcast::<TextInput>()
            && input.get_prop::<DefaultFocus>().0
        {
            return Some(input.area_pod().id());
        }
        pending.extend(widget.children());
    }
    None
}
/// A picker opened from a combobox control hangs from that control's bottom-left corner.
pub(super) fn place_picker(root: &RenderRoot, context: &Context, opener: Option<WidgetId>) {
    let Some(widget) = opener.and_then(|id| root.get_widget(id)) else {
        return;
    };
    if !widget
        .downcast::<Button>()
        .is_some_and(|button| button.get_prop::<PickerAnchor>().0)
    {
        return;
    }
    let corner = widget.ctx().window_transform() * widget.ctx().border_box().origin();
    let height = widget.ctx().border_box().height();
    let _ = context.proxy.message(Invocation {
        action: Action::PlacePicker {
            x: corner.x.max(0.).round() as u32,
            y: (corner.y + height).max(0.).round() as u32,
        },
        generation: context.generation,
    });
}
/// The first focusable problem wedge in the window, in reading order.
pub(super) fn first_wedge(root: &RenderRoot) -> Option<WidgetId> {
    let mut pending = vec![root.get_layer_root(0)];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() || widget.ctx().is_disabled() {
            continue;
        }
        if widget
            .downcast::<Wedge>()
            .is_some_and(|wedge| wedge.inner().is_control())
        {
            return Some(widget.id());
        }
        pending.extend(widget.children().into_iter().rev());
    }
    None
}
