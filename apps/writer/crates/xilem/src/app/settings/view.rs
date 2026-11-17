//! Settings: a modal side sheet over the dimmed, inert workspace, which still shows each change.
//! Preferences sit in groups under Section heads with their controls on the right; changes apply
//! at once. The sheet takes focus when it opens, gives it back when it closes, and Esc closes it.
use super::*;
use crate::app::keyboard::{DefaultFocus, ModalSurface};
use crate::presentation::group::dialog;
use crate::presentation::pane;
use crate::presentation::print::page_heading;
use crate::presentation::segments::{segment, segments_on_paper};
use crate::presentation::slip::slip;
use crate::presentation::tokens::size;
use crate::presentation::{Icon, label, print, prose, space, text_button};
use recite_writer_app::COMPACT_WINDOW;
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, Dim},
    style::Style,
    view::{CrossAxisAlignment, FlexSpacer, flex_col, flex_row, indexed_stack, sized_box},
};

/// The sheet's width: 360px, or the window less a margin either side below the comfortable
/// minimum and at 200%, so the sheet never runs past the window; its rows reflow inside it.
pub(in crate::app) fn sheet_width(app: &App) -> f64 {
    let scale = f64::from(crate::presentation::scale(&app.preferences.config.writer));
    if app.viewport.width < COMPACT_WINDOW * scale || scale >= 2. {
        (app.viewport.width - 2. * space::KINDS).max(0.)
    } else {
        size::SIDE_SHEET * scale
    }
}

pub(in crate::app) fn settings(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let width = sheet_width(app) - 2. * space::KINDS;
    dialog(
        "Settings",
        slip(
            &app.theme.palette,
            sized_box(
                pane(
                    "Settings",
                    flex_col((
                        flex_row((
                            page_heading(scale, "Settings"),
                            FlexSpacer::Flex(1.),
                            print::icon_action(
                                scale,
                                "Done with settings",
                                Icon::Close,
                                &app.theme.palette,
                                false,
                                |app: &mut App| app.close_settings(),
                            )
                            .prop(DefaultFocus(true)),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .width(width.px()),
                        pages(app),
                        indexed_stack((personal(app, width), keyboard(app, width)))
                            .active(usize::from(app.settings.keyboard)),
                        app.preferences
                            .error
                            .as_ref()
                            .map(|error| prose(scale, error.clone())),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::KINDS.px())
                    .padding(space::KINDS.px()),
                )
                .constrain_horizontal(true),
            )
            // The slip's 1px edge sits inside the sheet's width.
            .dims(((sheet_width(app) - 2.).px(), Dim::Stretch))
            .prop(ModalSurface(true)),
        ),
    )
}
mod keyboard;
mod personal;
use keyboard::keyboard;
use personal::personal;

#[cfg(test)]
mod tests;

/// Preferences and shortcuts: one segmented choice of page.
fn pages(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    segments_on_paper(
        &app.theme.palette,
        [
            ("Personal preferences", false),
            ("Keyboard shortcuts", true),
        ]
        .map(|(title, keys)| {
            segment(
                scale,
                &app.theme.palette,
                title,
                app.settings.keyboard == keys,
                false,
                move |app: &mut App| app.settings.keyboard = keys,
            )
        }),
    )
}
