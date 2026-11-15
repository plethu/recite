//! Native view constructors make UI text scale explicit at the rendering boundary.
use xilem::{
    WidgetView,
    core::MessageResult,
    masonry::core::ArcStr,
    view::{self, Button, Label, PointerButton, Prose},
};
use xilem::{
    masonry::{layout::AsUnit, parley::LineHeight},
    style::Style,
};

/// Body text: Plex Sans 14/20. A size override keeps the same proportional leading.
pub(crate) fn label(scale: f32, text: impl Into<ArcStr>) -> Label {
    let (size, line) = super::tokens::Type::Body.metrics();
    view::label(text)
        .font(super::print::font(
            recite_writer_session::typography::FONT_UI,
        ))
        .text_size(size * scale)
        .line_height(LineHeight::FontSizeRelative(line / size))
}
pub(crate) fn prose<State, Action>(scale: f32, text: impl Into<ArcStr>) -> Prose<State, Action> {
    view::prose(text).text_size(super::tokens::Type::Body.size(scale))
}
pub(crate) fn paragraph<State: 'static, Action: 'static>(
    scale: f32,
    text: impl Into<ArcStr>,
    width: f64,
) -> impl WidgetView<State, Action> {
    // A row resolves the reading width before measuring wrapped text height.
    // A column otherwise supplies its full width to the pinned Flex measurer.
    view::flex_row((prose(scale, text).width(width.px()),))
}
pub(crate) fn text_button<State: 'static>(
    scale: f32,
    text: impl Into<ArcStr>,
    callback: impl Fn(&mut State) + Send + Sync + 'static,
) -> Button<
    State,
    (),
    impl Fn(&mut State, Option<PointerButton>) -> MessageResult<()> + Send + Sync + 'static,
    Label,
> {
    view::button(super::tokens::Type::Control.label(scale, text), callback)
}
