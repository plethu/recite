//! Document toolbar, recovery location and guarded close presentation.
use super::*;
use crate::WriterLayout;
use crate::presentation::print;
use xilem::{masonry::properties::LineBreaking, view::portal};
mod menu;
pub(super) use menu::panel;

mod header;
pub(super) use header::{focus_header, project_header};

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
                label(scale, state.document_caption())
                    .prop(LineBreaking::WordWrap),
                (state.session.project().is_some() && !ordinary_exit).then(|| {
                    paragraph(scale, "Save all and close checks and saves pending prose and project drafts. Close and keep recovery keeps pending edits recoverable without saving them to the source files.", (state.viewport.width - 80.).min(760.))
                }),
                crate::localisation::close_details(state),
                flex_row((
                    text_button(scale, "Keep editing", Writer::keep_editing),
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
