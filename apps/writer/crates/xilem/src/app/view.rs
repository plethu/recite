mod chrome;
use super::*;
use crate::presentation::group::named;
use chrome::app_bar;
use xilem::{
    WidgetView,
    masonry::layout::{AsUnit, UnitPoint},
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, indexed_stack, sized_box, zstack},
};
pub(super) fn app(app: &mut App) -> impl WidgetView<App> + use<> {
    if let Some(writer) = &mut app.writer {
        writer.viewport.width = app.viewport.width;
    }
    let slot = slot::writer_slot(app);
    keyboard::host(
        app,
        xilem::core::fork(
            xilem::view::resize_observer(
                |app: &mut App, size| app.viewport = size,
                flex_col((
                    app_bar(app),
                    zstack((
                        named(
                            "Writer workspace",
                            indexed_stack((welcome::panel(app), slot, settings::settings(app)))
                                .active(if app.settings_open {
                                    2
                                } else {
                                    usize::from(!app.show_welcome)
                                }),
                        )
                        .disabled(app.picker.is_some()),
                        indexed_stack((
                            picker::view::picker(app),
                            sized_box(flex_col(())).dims((0.px(), 0.px())),
                        ))
                        .active(usize::from(app.picker.is_none())),
                    ))
                    .alignment(UnitPoint::CENTER)
                    .flex(1.),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
                .background_color(app.theme.palette.canvas),
            ),
            opening::worker(),
        ),
    )
}
