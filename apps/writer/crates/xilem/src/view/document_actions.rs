//! Document toolbar, recovery location and guarded close presentation.
use super::*;
use crate::presentation::print;
use xilem::{
    masonry::properties::LineBreaking,
    view::{GridExt, GridTrackSize, grid, portal},
};
mod menu;
pub(super) use menu::panel;

pub(super) fn project_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    let scale = state.ui_scale();
    let wide = state.viewport.width - state.navigation_width() - 32. >= 760. * f64::from(scale);
    grid((
        history_buttons(state).grid_pos(0, 0),
        flex_row((
            print::action(
                scale,
                "Return to writing",
                state.phase != Phase::Writing,
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
    .background_color(state.palette.surface)
}

pub(super) fn focus_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), document_caption(state)),
        temporary_notice(state),
        flex_row((
            text_button(state.ui_scale(), "Exit focus", Writer::toggle_focus).disabled(
                state.phase != Phase::Writing || state.navigation.busy() || state.work.busy(),
            ),
            state.session.project().map(|_| {
                text_button(state.ui_scale(), "Save file", Writer::save).disabled(
                    state.phase != Phase::Writing
                        || state.navigation.busy()
                        || state.work.busy()
                        || state.localisation.saving()
                        || state.source_repair_pending(),
                )
            }),
            text_button(state.ui_scale(), "Close", Writer::request_close)
                .disabled(state.phase != Phase::Writing),
            copy_link(state),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn document_caption(state: &Writer) -> String {
    format!(
        "{}{}",
        state.session.model().document().key(),
        if state.session.current_dirty() {
            " · Unsaved"
        } else {
            ""
        }
    )
}
fn temporary_notice(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    state.session.project().is_none().then(|| {
        label(
            state.ui_scale(),
            "Temporary example · changes are discarded when this window closes",
        )
    })
}

fn history_buttons(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    flex_row((
        print::action(
            scale,
            "Back",
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(false).is_none(),
            |state: &mut Writer| state.history_step(false),
        ),
        print::action(
            scale,
            "Forward",
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(true).is_none(),
            |state: &mut Writer| state.history_step(true),
        ),
        print::meta(scale, document_caption(state))
            .prop(LineBreaking::WordWrap)
            .flex(1.),
        copy_link(state),
    ))
    .gap(8.px())
}
pub(super) fn copy_link(state: &Writer) -> impl WidgetView<Writer> + use<> {
    text_button(state.ui_scale(), "Copy link", Writer::copy_link)
        .disabled(state.phase != Phase::Writing || state.project_action_blocked())
}

pub(super) fn recovery_copy(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let scale = state.ui_scale();
    state
        .recovery_copy
        .as_ref()
        .filter(|_| state.phase == Phase::Writing)
        .map(|path| {
            flex_col((
                text_button(
                    scale,
                    if state.recovery_details {
                        "Hide recovery copy location"
                    } else {
                        "Show recovery copy location"
                    },
                    |state: &mut Writer| state.recovery_details = !state.recovery_details,
                ),
                state.recovery_details.then(|| {
                    sized_box(prose(scale, path.display().to_string()))
                        .width((state.script_width() + state.navigation_width() + 20.).px())
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px())
        })
}

pub(super) fn close_prompt(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let scale = state.ui_scale();
    let ordinary_exit = state.phase == Phase::ConfirmExit;
    matches!(
        state.phase,
        Phase::ConfirmClose | Phase::ConfirmExit | Phase::Closing
    )
    .then(|| {
        sized_box(
            flex_col((
                label(
                    scale,
                    if state.phase == Phase::Closing {
                        "Finishing recovery before closing"
                    } else if ordinary_exit {
                        "Close Recite Writer?"
                    } else if state.session.project().is_some() {
                        "Keep your unsaved work before closing"
                    } else {
                        "Example edits are temporary and will be discarded"
                    },
                ),
                label(scale, document_caption(state))
                    .prop(LineBreaking::WordWrap),
                (state.session.project().is_some() && !ordinary_exit).then(|| {
                    paragraph(scale, "Save all and close checks and saves pending prose and project drafts. Close and keep recovery keeps pending edits recoverable without saving them to the source files.", (state.viewport.width - 80.).min(760.))
                }),
                crate::localisation::close_details(state),
                flex_row((
                    text_button(scale, "Keep editing", |state: &mut Writer| {
                        state.phase = Phase::Writing;
                        state.work.keep_editing();
                        state.localisation.keep_editing();
                    }),
                    (state.session.project().is_some()
                        && !ordinary_exit
                        && state.phase != Phase::Closing)
                        .then(|| {
                            text_button(scale, "Save all and close", Writer::save_and_close)
                                .disabled(
                                    state.work.busy()
                                        || state.project_jobs_busy()
                                        || state.source_repair_pending(),
                                )
                        }),
                    text_button(
                        scale,
                        if ordinary_exit {
                            "Close Writer"
                        } else if state.session.project().is_some() {
                            "Close and keep recovery"
                        } else {
                            "Close and discard example changes"
                        },
                        Writer::close_with_recovery,
                    )
                    .disabled(state.work.busy() || state.project_jobs_busy()),
                ))
                .gap(8.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .padding(12.px())
        .background_color(state.palette.surface)
    })
}

pub(super) fn close_view(state: &Writer) -> impl WidgetView<Writer> + use<> {
    portal(
        flex_col((
            xilem::view::FlexSpacer::Fixed(48.px()),
            flex_row((sized_box(flex_col((close_prompt(state),))).width(
                (state.viewport.width - 48.)
                    .min(720. * f64::from(state.ui_scale()))
                    .px(),
            ),))
            .main_axis_alignment(xilem::view::MainAxisAlignment::Center),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .padding(24.px()),
    )
    .constrain_horizontal(true)
    .background_color(state.palette.inset)
}
