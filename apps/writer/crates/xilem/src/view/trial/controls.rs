//! Submission stays outside the independently scrolling conversation and next-run form.
use super::*;
use xilem::masonry::kurbo::Axis;

pub(super) fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let wide = state.content_width() >= 600. * f64::from(scale);
    flex_col((
        grid((
            flex_row((
                label(scale, "Preview").text_size(18. * scale),
                submission(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(16.px())
            .grid_pos(0, 0),
            flex_row((
                print::action(
                    scale,
                    "Return to writing",
                    state.phase != Phase::Writing,
                    Writer::return_from_trial,
                ),
                print::action(
                    scale,
                    if state.configuring_trial() {
                        "Hide preview inputs"
                    } else {
                        "Configure preview"
                    },
                    state.phase != Phase::Writing,
                    |state: &mut Writer| {
                        if state.configuring_trial() {
                            state.close_trial_inputs();
                        } else {
                            state.configure_trial();
                        }
                    },
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
        .gap(8.px()),
        feedback::panel(state),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(8.px())
}

fn submission(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing || state.session.read_only();
    let unfinished = state.unfinished_trial_edit().is_some();
    let applying = state.navigation.applying_preview();
    flex_row((
        print::primary_action(
            scale,
            if applying {
                "Applying edit…"
            } else if unfinished {
                "Apply edit and preview"
            } else {
                state
                    .trial
                    .submission_caption(state.session.model().preview_page().is_some())
            },
            &state.palette,
            closing
                || state.trial.busy()
                || state.navigation.busy()
                || state.work.busy()
                || state.localisation.preview_blocked()
                || state.reading.script().is_err()
                || state.reading.script().is_ok_and(<[_]>::is_empty),
            move |state: &mut Writer| {
                if unfinished {
                    state.apply_trial_edit()
                } else {
                    state.start_trial()
                }
            },
        ),
        (state.trial.busy() || applying).then(|| {
            text_button(
                scale,
                "Cancel preparation",
                Writer::cancel_trial_preparation,
            )
            .disabled(closing || (!applying && !state.trial.can_cancel()))
        }),
    ))
    .direction(if state.content_width() >= 360. * f64::from(scale) {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
