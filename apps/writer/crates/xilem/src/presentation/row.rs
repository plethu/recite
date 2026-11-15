//! List rows: a row tall, quiet at rest, the selected one on the second-ink plate with a 2px ink
//! bar down its leading edge and the accessible `selected` state, so no row spells its selection
//! into its name. Rows are native buttons: press with the pointer, Space or Enter, or the Click
//! action.
#[cfg(test)]
mod tests;
use super::control_style::{ControlStyle, styled};
use super::material::{Bar, Edge, bar};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    core::MessageResult,
    masonry::{core::ArcStr, widgets::Button as ButtonWidget},
    peniko::Color,
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

/// Dress a row view in the list look: the plate and leading ink bar while `selected`.
pub(crate) fn listed<S: 'static, R: WidgetView<S, Widget = ButtonWidget>>(
    palette: &Palette<Color>,
    selected: bool,
    row: R,
) -> impl WidgetView<S> + use<S, R> {
    bar(
        selected.then_some(Bar::solid(palette.ink, Edge::Leading, 0., 0.)),
        styled(
            if selected {
                ControlStyle::RowSelected
            } else {
                ControlStyle::Row
            },
            row,
        ),
    )
}
