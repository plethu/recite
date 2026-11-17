mod chrome;
use super::*;
use crate::presentation::{
    Type,
    group::named,
    material::{backdrop, desk},
    space,
    tokens::size,
};
use crate::workspaces::rename::inline as rename;
use chrome::app_bar;
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, Dim, UnitPoint},
    style::{Padding, Style},
    view::{
        CrossAxisAlignment, FlexExt, FlexSpacer, flex_col, flex_row, indexed_stack, sized_box,
        zstack,
    },
};
pub(super) fn app(app: &mut App) -> impl WidgetView<App> + use<> {
    let width = app.viewport.width;
    if let Some(writer) = &mut app.writer {
        writer.viewport.width = width;
    }
    let slot = slot::writer_slot(app);
    let scale = f64::from(crate::presentation::scale(&app.preferences.config.writer));
    // The status bar's height: its vertical padding and one Meta line.
    let status_bar = if app.writer.is_some() && !app.show_welcome() {
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
                    zstack((
                        flex_col((
                            // The close decision dims the document within the writer; the tab
                            // row above it dims with it.
                            zstack((
                                app_bar(app),
                                app.writer
                                    .as_ref()
                                    .is_some_and(|writer| {
                                        matches!(
                                            writer.phase(),
                                            crate::Phase::ConfirmClose
                                                | crate::Phase::ConfirmExit
                                                | crate::Phase::Closing
                                        )
                                    })
                                    .then(|| {
                                        backdrop(
                                            app.theme.palette.backdrop,
                                            app.theme.screen.clone(),
                                            sized_box(flex_col(())).dims(Dim::Stretch),
                                        )
                                    }),
                            )),
                            zstack((
                                named(
                                    "Writer workspace",
                                    indexed_stack((welcome::panel(app), slot))
                                        .active(usize::from(!app.show_welcome())),
                                )
                                .disabled(app.picker.is_some()),
                                // Settings is a side sheet over the live workspace, clear of the
                                // status bar. A stack keeps the layer in place while it is
                                // closed, so the layers above it keep theirs.
                                indexed_stack((
                                    flex_row((FlexSpacer::Flex(1.), settings::settings(app)))
                                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                        // Below the toolbar too, so it covers none of it.
                                        .padding(Padding {
                                            top: ((size::PICKER_TOP - size::ROW)
                                                * f64::from(crate::presentation::scale(
                                                    &app.preferences.config.writer,
                                                )))
                                            .px(),
                                            bottom: status_bar.px(),
                                            ..Padding::all(0.px())
                                        })
                                        .dims(Dim::Stretch),
                                    sized_box(flex_col(())).dims((0.px(), 0.px())),
                                ))
                                .active(usize::from(!app.settings_open()))
                                .dims(if app.settings_open() {
                                    Dim::Stretch
                                } else {
                                    Dim::Auto
                                }),
                            ))
                            .alignment(UnitPoint::CENTER)
                            .flex(1.),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(0.px())
                        .dims(Dim::Stretch),
                        // A picker's or rename slip's backdrop dims the window above the status
                        // bar, the tab row included; the bar carries state, so its words keep their
                        // contrast. The layer is empty and takes no pointer events otherwise.
                        modal_layer(app.picker.is_some(), status_bar, picker_layer(app, scale)),
                        modal_layer(
                            app.writer.as_ref().is_some_and(rename::floats),
                            status_bar,
                            slot::rename_slot(app),
                        ),
                    ))
                    .alignment(UnitPoint::CENTER)
                    .dims(Dim::Stretch),
                )
                .background_color(app.theme.palette.canvas),
            ),
            opening::worker(),
        ),
    )
}

/// A layer over the window above its `status_bar` that fills it while `open` and is an empty
/// corner otherwise, so it never takes the pointer events of the window beneath.
fn modal_layer<V: WidgetView<App>>(
    open: bool,
    status_bar: f64,
    layer: V,
) -> impl WidgetView<App> + use<V> {
    indexed_stack((
        sized_box(sized_box(layer).dims(Dim::Stretch))
            .dims(Dim::Stretch)
            .padding(Padding {
                bottom: status_bar.px(),
                ..Padding::all(0.px())
            }),
        sized_box(flex_col(())).dims((0.px(), 0.px())),
    ))
    .active(usize::from(!open))
    .dims(if open { Dim::Stretch } else { Dim::Auto })
}

/// An open picker over the backdrop: below the tab row at a fixed height from the window's top
/// edge, or hanging from the combobox that opened it; its layer ends above the status bar.
fn picker_layer(app: &App, scale: f64) -> impl WidgetView<App> + use<> {
    let placement = picker::view::placement(app);
    backdrop(
        app.theme.palette.backdrop,
        app.theme.screen.clone(),
        // A stack bounds the picker by the layer, so it scrolls its results rather than overflow
        // at 200%.
        zstack((
            sized_box(picker::view::picker(app)).padding(placement.map_or(
                Padding::top((size::PICKER_TOP * scale).px()),
                |(left, top)| Padding {
                    left: left.px(),
                    top: top.px(),
                    ..Padding::all(0.px())
                },
            )),
        ))
        .alignment(if placement.is_some() {
            UnitPoint::TOP_LEFT
        } else {
            UnitPoint::TOP
        })
        .dims(Dim::Stretch),
    )
}

#[cfg(test)]
mod tests;
