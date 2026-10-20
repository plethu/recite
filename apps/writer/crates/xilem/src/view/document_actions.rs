//! Document toolbar, recovery location and guarded close presentation.
use super::*;

pub(super) fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    flex_col((
        flex_row((
            label(scale, "Recite Writer").text_size(28. * scale),
            history_buttons(state),
        ))
        .gap(24.px()),
        label(
            scale,
            format!(
                "{}{}",
                state.session.model().document().key().as_str(),
                if state.session.current_dirty() {
                    " · Unsaved"
                } else {
                    ""
                }
            ),
        ),
        state.session.project().is_none().then(|| {
            label(
                scale,
                "Temporary example · changes are discarded when this window closes",
            )
        }),
        flex_row((
            state.session.project().is_some().then(|| {
                text_button(scale, "Save file", |state: &mut Writer| {
                    state.save();
                })
                .disabled(
                    closing
                        || state.work.busy()
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
            text_button(scale, "Close", Writer::request_close).disabled(closing),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
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
    ))
    .gap(8.px())
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
    matches!(state.phase, Phase::ConfirmClose | Phase::ConfirmExit).then(|| {
        sized_box(
            flex_col((
                label(
                    scale,
                    if ordinary_exit {
                        "Close Recite Writer?"
                    } else if state.session.project().is_some() {
                        "Keep your unsaved work before closing"
                    } else {
                        "Example edits are temporary and will be discarded"
                    },
                ),
                flex_row((
                    text_button(scale, "Keep editing", |state: &mut Writer| {
                        state.phase = Phase::Writing;
                        state.work.keep_editing();
                    }),
                    (state.session.project().is_some() && !ordinary_exit).then(|| {
                        text_button(scale, "Save all and close", Writer::save_and_close)
                            .disabled(state.work.busy() || state.source_repair_pending())
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
                    .disabled(state.work.busy()),
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
