//! The head of a tool or Localise sheet: the page's title in Display, the sheet's own actions,
//! and a named close icon that returns to writing.
use super::{Icon, Type, print, space};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    masonry::{core::ArcStr, layout::AsUnit, properties::LineBreaking, widgets::Flex},
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, FlexExt, FlexSequence, flex_row},
};

/// What closes a sheet: the action, and whether it is unavailable now.
pub(crate) struct Close<F> {
    pub disabled: bool,
    pub action: F,
}

pub(crate) fn sheet_head<
    S: 'static,
    T: Into<ArcStr>,
    A: FlexSequence<S> + Send + Sync,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    scale: f32,
    palette: &Palette<Color>,
    title: T,
    actions: A,
    close: Close<F>,
) -> impl WidgetView<S, Widget = Flex> + use<S, T, A, F> {
    flex_row((
        Type::Display
            .label(scale, title)
            .prop(LineBreaking::WordWrap)
            .flex(1.),
        actions,
        print::icon_action(
            scale,
            "Return to writing",
            Icon::Close,
            palette,
            close.disabled,
            close.action,
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::TEXT.px())
}
