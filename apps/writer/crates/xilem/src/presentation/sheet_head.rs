//! The head of a tool or Localise sheet: the page's title in Display, the sheet's own actions,
//! and a named close icon that returns to writing. On a narrow sheet the actions take a second
//! row under the title, so neither squeezes the other.
use super::{Icon, Type, print, space};
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    masonry::{core::ArcStr, layout::AsUnit, properties::LineBreaking, widgets::Flex},
    peniko::Color,
    style::Style,
    view::{CrossAxisAlignment, FlexExt, FlexSequence, flex_col, flex_row},
};

/// What closes a sheet: the action, and whether it is unavailable now.
pub(crate) struct Close<F> {
    pub disabled: bool,
    pub action: F,
}

/// How a sheet head sets its parts: the interface scale, and whether the actions take a second
/// row.
#[derive(Clone, Copy)]
pub(crate) struct Layout {
    pub scale: f32,
    pub stacked: bool,
}

pub(crate) fn sheet_head<
    S: 'static,
    T: Into<ArcStr>,
    A: FlexSequence<S> + Send + Sync,
    F: Fn(&mut S) + Send + Sync + 'static,
>(
    Layout { scale, stacked }: Layout,
    palette: &Palette<Color>,
    (title, actions): (T, A),
    close: Close<F>,
) -> impl WidgetView<S, Widget = Flex> + use<S, T, A, F> {
    let title = Type::Display
        .label(scale, title)
        .prop(LineBreaking::WordWrap)
        .flex(1.);
    let close = print::icon_action(
        scale,
        "Return to writing",
        Icon::Close,
        palette,
        close.disabled,
        close.action,
    );
    let (inline, below) = if stacked {
        (None, Some(actions))
    } else {
        (Some(actions), None)
    };
    flex_col((
        flex_row((title, inline, close))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::TEXT.px()),
        below.map(|actions| {
            flex_row(actions)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
