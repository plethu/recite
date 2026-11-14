//! Adaptive document navigation keeps project tools available without displacing their content.
use super::*;
use crate::presentation::Icon;
use xilem::view::{GridExt, GridTrackSize, grid};

pub(in crate::view) fn project_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy();
    let scale = state.ui_scale();
    let wide = state.content_width()
        >= if state.compact_chrome() { 520. } else { 760. } * f64::from(scale);
    grid((
        history_buttons(state).grid_pos(0, 0),
        flex_row((
            print::action(
                scale,
                "Return to writing",
                state.phase() != Phase::Writing,
                Writer::return_to_writing,
            ),
            print::action(
                scale,
                "Save all",
                blocked
                    || state.session.read_only()
                    || state.source_repair_pending()
                    || state.external_resolution_pending()
                    || state.project_jobs_busy(),
                Writer::save_all,
            ),
            print::action(
                scale,
                "Close",
                blocked || state.project_jobs_busy(),
                Writer::request_close,
            ),
        ))
        .gap(8.px())
        .grid_pos(u16::from(wide), u16::from(!wide)),
    ))
    .columns([
        GridTrackSize::FRACTION,
        if wide {
            GridTrackSize::Auto
        } else {
            GridTrackSize::Fixed(0.px())
        },
    ])
    .rows([GridTrackSize::Auto; 2])
    .gap(4.px())
    .padding(xilem::style::Padding::from_vh(4.px(), 16.px()))
}

pub(in crate::view) fn focus_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), state.document_caption()),
        temporary_notice(state),
        flex_row((
            text_button(state.ui_scale(), "Exit focus", Writer::toggle_focus).disabled(
                state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy(),
            ),
            state.session.project().map(|_| {
                text_button(state.ui_scale(), "Save file", Writer::save).disabled(
                    state.phase() != Phase::Writing
                        || state.navigation.busy()
                        || state.work.busy()
                        || state.localisation.saving()
                        || state.source_repair_pending(),
                )
            }),
            text_button(state.ui_scale(), "Close", Writer::request_close)
                .disabled(state.phase() != Phase::Writing),
            copy_link(state),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn temporary_notice(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    state.session.project_root().is_none().then(|| {
        label(
            state.ui_scale(),
            "Temporary example · changes are discarded when this window closes",
        )
    })
}

fn history_buttons(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing;
    flex_row((
        print::icon_action(
            scale,
            "Back",
            Icon::Back,
            &state.palette,
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(false).is_none(),
            |state: &mut Writer| state.history_step(false),
        ),
        print::icon_action(
            scale,
            "Forward",
            Icon::Forward,
            &state.palette,
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(true).is_none(),
            |state: &mut Writer| state.history_step(true),
        ),
        (!state.compact_chrome()).then(|| {
            print::meta(scale, state.document_caption())
                .prop(LineBreaking::WordWrap)
                .flex(1.)
        }),
        print::icon_action(
            scale,
            "Copy link",
            Icon::Link,
            &state.palette,
            closing || state.project_action_blocked(),
            Writer::copy_link,
        ),
    ))
    .gap(8.px())
}
fn copy_link(state: &Writer) -> impl WidgetView<Writer> + use<> {
    text_button(state.ui_scale(), "Copy link", Writer::copy_link)
        .disabled(state.phase() != Phase::Writing || state.project_action_blocked())
}
