//! A failed restart stays visible beside its action, even with the old run scrolled into view.
use super::*;
use crate::{presentation::group, trial::Review};
use xilem::masonry::kurbo::Axis;

pub(super) fn panel(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let message = state
        .trial
        .feedback(state.session.model().preview_page().is_some())?;
    let diagnostic =
        state.trial.failed() && matches!(state.trial.settings.review, Review::Diagnostic);
    Some(
        flex_col((
            summary(state, message, diagnostic),
            diagnostic.then(|| details(state)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}

fn summary(
    state: &Writer,
    message: &'static str,
    diagnostic: bool,
) -> impl WidgetView<Writer> + use<> {
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
                text_button(
                    scale,
                    if diagnostic {
                        "Hide preview error"
                    } else {
                        "Review current inputs"
                    },
                    move |state: &mut Writer| {
                        if diagnostic {
                            state.configure_trial();
                        } else {
                            state.review_trial_inputs();
                        }
                    },
                )
                .disabled(state.phase != Phase::Writing || state.session.read_only())
            }),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
        .prop(PaneEntry(diagnostic)),
    )
}

fn details(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    named(
        "Preview preparation diagnostic",
        flex_col((portal(paragraph(
            scale,
            format!(
                "Scalar inputs were valid when last reviewed. Failed attempt:\n{}",
                state.trial.failure().unwrap_or_default()
            ),
            state.content_width() - 16.,
        ))
        .constrain_horizontal(true)
        .height((72. * f64::from(scale)).px()),)),
    )
}
