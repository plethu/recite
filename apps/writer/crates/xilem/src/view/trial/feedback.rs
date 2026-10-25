//! A failed restart stays visible beside its action, even with the old run scrolled into view.
use super::*;
use crate::{attention::EntryContext, presentation::group};
use xilem::masonry::kurbo::Axis;

pub(super) fn panel(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let message = state
        .trial
        .feedback(state.session.model().preview_page().is_some())?;
    Some(summary(state, message))
}

fn summary(state: &Writer, message: &'static str) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let wide = state.content_width() >= 360. * f64::from(scale);
    group::status(
        message,
        flex_row((
            paragraph(
                scale,
                message,
                if wide {
                    state.content_width() - 220. * f64::from(scale)
                } else {
                    state.content_width()
                },
            ),
            state.trial.failed().then(|| {
                text_button(scale, "Review current inputs", Writer::review_trial_inputs)
                    .disabled(state.phase != Phase::Writing || state.session.read_only())
            }),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}

pub(super) fn details(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    named(
        "Preview preparation diagnostic",
        sized_box(
            flex_col((
                flex_row((
                    label(scale, "Previous attempt failed").text_size(18. * scale),
                    print::action(scale, "Hide preview error", false, Writer::configure_trial),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(16.px())
                .prop(PaneEntry(true))
                .prop(EntryContext(true)),
                paragraph(
                    scale,
                    state.trial.failure().unwrap_or_default().to_owned(),
                    state.content_width() - 48.,
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
        .padding(12.px())
        .background_color(state.palette.inset),
    )
}
