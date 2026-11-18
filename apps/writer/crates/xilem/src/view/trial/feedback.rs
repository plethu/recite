//! A failed restart stays visible beside its action, even with the old run scrolled into view.
use super::*;
use crate::presentation::space;
use crate::{attention::EntryContext, presentation::group};
use xilem::masonry::kurbo::Axis;

pub(super) fn panel(state: &Writer, width: f64) -> Option<impl WidgetView<Writer> + use<>> {
    let message = state.trial_feedback()?;
    Some(summary(state, message, width))
}

fn summary(state: &Writer, message: &'static str, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let wide = width >= 360. * f64::from(scale);
    group::status(
        message,
        flex_row((
            paragraph(
                scale,
                message,
                if wide {
                    width - 220. * f64::from(scale)
                } else {
                    width
                },
            ),
            state.trial.failed().then(|| {
                text_button(scale, "Review current inputs", Writer::review_trial_inputs)
                    .disabled(state.phase() != Phase::Writing || state.session.read_only())
            }),
            state.unfinished_trial_edit().is_some().then(|| {
                text_button(
                    scale,
                    "Return to unfinished edit",
                    Writer::return_to_trial_edit,
                )
                .disabled(state.phase() != Phase::Writing || state.session.read_only())
            }),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px())
        .prop(PaneEntry(state.unfinished_trial_edit().is_some()))
        .prop(EntryContext(true)),
    )
}

pub(super) fn details(state: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    named(
        "Preview preparation diagnostic",
        sized_box(
            flex_col((
                flex_row((
                    print::meta(scale, "Error details").color(state.palette.muted),
                    print::action(scale, "Hide preview error", false, Writer::configure_trial),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::KINDS.px())
                .prop(PaneEntry(true))
                .prop(EntryContext(true)),
                paragraph(
                    scale,
                    state.trial.failure().unwrap_or_default().to_owned(),
                    width - 2. * space::KINDS,
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        )
        .padding(space::KINDS.px())
        .background_color(state.palette.inset),
    )
}
