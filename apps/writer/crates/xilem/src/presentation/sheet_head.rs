//! The head of a tool or Localise sheet: the page's title in Display with an optional muted
//! caption, the sheet's own actions, and its link and named close icons, the close returning to
//! where it opened from. On a narrow sheet the actions take a second row under the title, so
//! neither squeezes the other.
use super::{Icon, Type, print, space};
use crate::attention::EntrySkip;
use recite_writer_session::palette::Palette;
use xilem::{
    WidgetView,
    masonry::{core::ArcStr, layout::AsUnit, properties::LineBreaking, widgets::Flex},
    peniko::Color,
    style::Style,
    view::{
        CrossAxisAlignment, FlexSequence, GridExt, GridTrackSize, MainAxisAlignment, flex_col,
        flex_row, grid,
    },
};

/// What closes a sheet: the action, and whether it and the sheet's link are unavailable now.
pub(crate) struct Close<F, L> {
    /// Where the icon returns to, as its accessible name, such as "Return to writing".
    pub name: &'static str,
    pub disabled: bool,
    pub action: F,
    /// Copies a link to the sheet: beside its title, as the galley's beat heading carries one.
    pub link: L,
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
    L: Fn(&mut S) + Send + Sync + 'static,
>(
    Layout { scale, stacked }: Layout,
    palette: &Palette<Color>,
    (title, caption, actions): (T, Option<ArcStr>, A),
    close: Close<F, L>,
) -> impl WidgetView<S, Widget = Flex> + use<S, T, A, F, L> {
    // The caption is one muted line under the title: what the sheet holds, never instructions.
    let title: ArcStr = title.into();
    let title = flex_col((
        // Focus enters at the heading of a page with nothing to act on yet, so it shows there.
        super::material::focus_outline(
            palette.accent,
            2.,
            super::group::heading(
                1,
                title.clone(),
                Type::Display
                    .label(scale, title)
                    .prop(LineBreaking::WordWrap),
            ),
        ),
        caption.map(|caption| {
            print::caption(scale, caption)
                .color(palette.muted)
                .prop(LineBreaking::WordWrap)
        }),
    ))
    .main_axis_alignment(MainAxisAlignment::Center)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px());
    let link = print::icon_action(
        scale,
        "Copy link",
        Icon::Link,
        palette,
        close.disabled,
        close.link,
    )
    .prop(EntrySkip(true));
    let close = print::icon_action(
        scale,
        close.name,
        Icon::Close,
        palette,
        close.disabled,
        close.action,
    )
    .prop(EntrySkip(true));
    // One grid either way: the actions move between the title's row and the row under it
    // without being rebuilt, so a control keeps its identity and focus as the sheet narrows.
    let actions = flex_row(actions)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::SIBLINGS.px());
    flex_col((grid((
        title.grid_pos(0, 0),
        actions.grid_pos(u16::from(!stacked), u16::from(stacked)),
        flex_row((link, close))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::UNIT.px())
            .grid_pos(2, 0),
    ))
    .columns([
        GridTrackSize::FRACTION,
        GridTrackSize::Auto,
        GridTrackSize::Auto,
    ])
    .rows(if stacked {
        vec![GridTrackSize::Auto; 2]
    } else {
        vec![GridTrackSize::Auto]
    })
    .gap(space::TEXT.px()),))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(0.px())
}
