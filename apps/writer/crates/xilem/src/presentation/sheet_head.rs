//! The head of a tool or Localise sheet: the page's title in Display with an optional muted
//! caption, the sheet's own actions, and a named close icon that returns to where it opened from. On a narrow sheet the actions take a second
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
    /// Where the icon returns to, as its accessible name, such as "Return to writing".
    pub name: &'static str,
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
    (title, caption, actions): (T, Option<ArcStr>, A),
    close: Close<F>,
) -> impl WidgetView<S, Widget = Flex> + use<S, T, A, F> {
    // The caption is one muted line under the title: what the sheet holds, never instructions.
    let title = flex_col((
        Type::Display
            .label(scale, title)
            .prop(LineBreaking::WordWrap),
        caption.map(|caption| {
            print::meta(scale, caption)
                .color(palette.muted)
                .prop(LineBreaking::WordWrap)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
    .flex(1.);
    let close = print::icon_action(
        scale,
        close.name,
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
