//! Document toolbar, recovery location and guarded close presentation.
use super::*;
use xilem::masonry::kurbo::Axis;

pub(super) fn project_header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase != Phase::Writing || state.work.busy() || state.navigation.busy();
    flex_col((
        label(
            state.ui_scale(),
            format!("Recite Writer · {}", document_caption(state)),
        ),
        flex_row((
            text_button(
                state.ui_scale(),
                "Return to writing",
                Writer::return_to_writing,
            )
            .disabled(state.phase != Phase::Writing),
            text_button(state.ui_scale(), "Save all", Writer::save_all).disabled(
                blocked
                    || state.session.read_only()
                    || state.source_repair_pending()
                    || state.external_resolution_pending()
                    || state.project_jobs_busy(),
            ),
            text_button(state.ui_scale(), "Close", Writer::request_close)
                .disabled(blocked || state.project_jobs_busy()),
            copy_link(state),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
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

pub(super) fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    flex_col((
        flex_row((
            label(scale, "Recite Writer").text_size(28. * scale),
            history_buttons(state),
        ))
        .direction(if state.viewport.width < 900. * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(24.px()),
        label(scale, document_caption(state)),
        temporary_notice(state),
        flex_row((
            state.session.project().is_some().then(|| {
                text_button(scale, "Save file", |state: &mut Writer| {
                    state.save();
                })
                .disabled(
                    closing
                        || state.work.busy()
                        || state.localisation.saving()
                        || state.navigation.busy()
                        || state.source_repair_pending(),
                )
            }),
            state.session.project().is_some().then(|| {
                text_button(scale, "Save all", |state: &mut Writer| {
                    state.save_all();
                })
                .disabled(
                    closing
                        || state.work.busy()
                        || state.localisation.saving()
                        || state.localisation.working()
                        || state.navigation.busy()
                        || state.source_repair_pending(),
                )
            }),
            state.session.project().is_some().then(|| {
                text_button(scale, "Open in editor", Writer::open_editor).disabled(
                    closing
                        || state.work.busy()
                        || state.navigation.busy()
                        || state.external.busy(),
                )
            }),
            (state.session.project().is_some() && state.editor.is_some()).then(|| {
                text_button(scale, "Use system default", Writer::open_system_editor).disabled(
                    closing
                        || state.work.busy()
                        || state.navigation.busy()
                        || state.external.busy(),
                )
            }),
        ))
        .gap(8.px()),
        flex_row((
            state
                .work
                .preparing_editor()
                .then(|| text_button(scale, "Cancel editor launch", Writer::cancel_editor_launch)),
            state.session.project().is_some().then(|| {
                text_button(scale, "Reload (keep recovery copy)", Writer::reload)
                    .disabled(closing || state.work.busy() || state.navigation.busy())
            }),
            state.session.project().is_some().then(|| {
                text_button(scale, "Compare disk changes", Writer::compare_disk_changes)
                    .disabled(state.project_action_blocked())
            }),
            text_button(scale, "Focus writing", Writer::toggle_focus)
                .disabled(closing || state.work.busy() || state.navigation.busy()),
            text_button(scale, "Close", Writer::request_close).disabled(closing),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
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
        text_button(scale, "Back", |state: &mut Writer| {
            state.history_step(false)
        })
        .disabled(
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(false).is_none(),
        ),
        text_button(scale, "Forward", |state: &mut Writer| {
            state.history_step(true)
        })
        .disabled(
            closing
                || state.work.busy()
                || state.navigation.busy()
                || state.navigation.history.step(true).is_none(),
        ),
        copy_link(state),
    ))
    .gap(8.px())
}
fn copy_link(state: &Writer) -> impl WidgetView<Writer> + use<> {
    text_button(
        state.ui_scale(),
        "Copy link to this view",
        Writer::copy_link,
    )
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
