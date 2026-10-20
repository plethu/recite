//! Native view constructors make UI text scale explicit at the rendering boundary.
use xilem::{
    core::MessageResult,
    masonry::core::ArcStr,
    view::{self, Button, Label, PointerButton, Prose},
};

pub(crate) fn label(scale: f32, text: impl Into<ArcStr>) -> Label {
    view::label(text).text_size(14. * scale)
}
pub(crate) fn prose<State, Action>(scale: f32, text: impl Into<ArcStr>) -> Prose<State, Action> {
    view::prose(text).text_size(14. * scale)
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
    view::button(label(scale, text), callback)
}
