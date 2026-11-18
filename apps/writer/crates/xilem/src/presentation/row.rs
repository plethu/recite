//! List rows: a row tall, quiet at rest, the selected one on the second-ink plate with its words
//! underscored in ink (see `marks::underscored`) and the accessible `selected` state, so no row
//! spells its selection into its name. Rows are native buttons: press with the pointer, Space or
//! Enter, or the Click action.
#[cfg(test)]
mod tests;
use super::control_style::{ControlStyle, styled};
use xilem::{
    WidgetView,
    core::MessageResult,
    masonry::{core::ArcStr, widgets::Button as ButtonWidget},
    view::{Button, PointerButton, button},
};

/// A row of `child` that calls `press` when pressed, marked selected while `selected` holds.
pub(crate) fn row<S: 'static, V: WidgetView<S>, F: Fn(&mut S) + Send + Sync + 'static>(
    selected: bool,
    child: V,
    press: F,
) -> Button<S, (), impl Fn(&mut S, Option<PointerButton>) -> MessageResult<()> + Send + 'static, V>
{
    button(child, press).selected(selected)
}

/// A pressable row outside any list, named `name` whatever its content shows, such as a beat
/// heading whose button says what pressing it does.
pub(crate) fn named_row<S: 'static, V: WidgetView<S>, F: Fn(&mut S) + Send + Sync + 'static>(
    name: impl Into<ArcStr>,
    child: V,
    press: F,
) -> Button<S, (), impl Fn(&mut S, Option<PointerButton>) -> MessageResult<()> + Send + 'static, V>
{
    button(child, press).accessible_label(name)
}

/// Dress a row view in the list look: the plate while `selected`. The row's words carry the ink
/// underscore themselves, so it runs under their own width.
pub(crate) fn listed<S: 'static, R: WidgetView<S, Widget = ButtonWidget>>(
    selected: bool,
    row: R,
) -> impl WidgetView<S> + use<S, R> {
    styled(
        if selected {
            ControlStyle::RowSelected
        } else {
            ControlStyle::Row
        },
        row,
    )
}
