mod chrome;
use super::*;
use crate::presentation::{
    Type,
    group::named,
    material::{backdrop, desk},
    space,
    tokens::size,
};
use chrome::app_bar;
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, Dim, UnitPoint},
    style::{Padding, Style},
    view::{CrossAxisAlignment, FlexExt, flex_col, indexed_stack, sized_box, zstack},
};
pub(super) fn app(app: &mut App) -> impl WidgetView<App> + use<> {
    let width = app.viewport.width;
    if let Some(writer) = &mut app.writer {
        writer.viewport.width = width;
    }
    let slot = slot::writer_slot(app);
    let scale = f64::from(crate::presentation::scale(&app.preferences.config.writer));
    // The status bar's height: its vertical padding and one Meta line.
    let status_bar = if app.writer.is_some() && !app.show_welcome() && !app.settings_open() {
        2. * space::UNIT
            + f64::from(Type::Meta.metrics().1)
                * f64::from(crate::presentation::scale(&app.preferences.config.writer))
    } else {
        0.
    };
    keyboard::host(
        app,
        xilem::core::fork(
            xilem::view::resize_observer(
                |app: &mut App, size| app.viewport = size,
                desk(
                    app.theme.grain.clone(),
                    flex_col((
                        app_bar(app),
                        zstack((
                            named(
                                "Writer workspace",
                                indexed_stack((welcome::panel(app), slot, settings::settings(app)))
                                    .active(if app.settings_open() {
                                        2
                                    } else {
                                        usize::from(!app.show_welcome())
                                    }),
                            )
                            .disabled(app.picker.is_some()),
                            indexed_stack((
                                // Pickers stay above the status bar at every interface scale, and
                                // their backdrop leaves the status bar readable.
                                sized_box(backdrop(
                                    app.theme.palette.backdrop,
                                    app.theme.screen.clone(),
                                    // A stack bounds the picker by the layer, so it
                                    // scrolls its results rather than overflow at 200%.
                                    zstack((sized_box(picker::view::picker(app)).padding(
                                        Padding::top(
                                            // Below the tab row, a picker opens at a fixed
                                            // height from the window's top edge.
                                            ((size::PICKER_TOP - size::ROW) * scale).px(),
                                        ),
                                    ),))
                                    .alignment(UnitPoint::TOP)
                                    .dims(Dim::Stretch),
                                ))
                                .padding(Padding {
                                    bottom: status_bar.px(),
                                    ..Padding::all(0.px())
                                })
                                .dims(Dim::Stretch),
                                sized_box(flex_col(())).dims((0.px(), 0.px())),
                            ))
                            .active(usize::from(app.picker.is_none()))
                            // Only an open picker's backdrop covers the workspace; otherwise the
                            // empty layer must not sit over it and take its pointer events.
                            .dims(if app.picker.is_some() {
                                Dim::Stretch
                            } else {
                                Dim::Auto
                            }),
                        ))
                        .alignment(UnitPoint::CENTER)
                        .flex(1.),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(0.px()),
                )
                .background_color(app.theme.palette.canvas),
            ),
            opening::worker(),
        ),
    )
}
