mod chrome;
use super::*;
use chrome::app_bar;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, indexed_stack},
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
                    indexed_stack((
                        welcome::panel(app),
                        slot,
                        settings::settings(app),
                        picker::view::picker(app),
                    ))
                    .active(if app.picker.is_some() {
                        3
                    } else if app.settings_open {
                        2
                    } else {
                        usize::from(!app.show_welcome)
                    })
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
