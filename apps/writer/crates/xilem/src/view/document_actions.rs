//! Document toolbar, recovery location and guarded close presentation.
use super::*;
use crate::WriterLayout;
use crate::app::keyboard::{DefaultFocus, ModalSurface};
use crate::presentation::Type;
use crate::presentation::group::dialog;
use crate::presentation::material::{Texture, backdrop, screen};
use crate::presentation::print;
use crate::presentation::slip::slip;
use crate::presentation::space;
use crate::presentation::tokens::size;
use xilem::masonry::layout::Dim;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{FlexSpacer, MainAxisAlignment};
mod menu;
pub(super) use menu::panel;

mod header;
pub(super) use header::focus_header;

pub(super) fn recovery_copy(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let scale = state.ui_scale();
    state
        .recovery_copy
        .as_ref()
        .filter(|_| state.phase() == Phase::Writing)
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
            .gap(space::SIBLINGS.px())
        })
}

/// The decision before closing with unsaved work: a slip over the dimmed document, titled in
/// Section type, with "Keep editing" focused first and Esc keeping the document open.
pub(super) fn close_decision(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let scale = state.ui_scale();
    let ordinary_exit = state.phase() == Phase::ConfirmExit;
    let project = state.session.project().is_some();
    let width = (size::DECISION_WIDTH * f64::from(scale))
        .min(state.viewport.width - 2. * space::GROUPS)
        - 2. * space::GROUPS;
    matches!(
        state.phase(),
        Phase::ConfirmClose | Phase::ConfirmExit | Phase::Closing
    )
    .then(|| {
        let title = if state.phase() == Phase::Closing {
            "Finishing recovery before closing"
        } else if ordinary_exit {
            "Close Recite Writer?"
        } else if project {
            "Keep your unsaved work before closing"
        } else {
            "Example edits are temporary and will be discarded"
        };
        let saving = project && !ordinary_exit && state.phase() != Phase::Closing;
        backdrop(
            state.palette.backdrop,
            screen(Texture::default(), state.palette.screen),
            flex_col((dialog(
                title,
                slip(
                    &state.palette,
                    sized_box(
                        flex_col((
                            Type::Section
                                .label(scale, title)
                                .prop(LineBreaking::WordWrap)
                                .width(width.px()),
                            print::meta(scale, state.document_caption())
                                .color(state.palette.muted)
                                .prop(LineBreaking::WordWrap)
                                .width(width.px()),
                            saving.then(|| {
                                paragraph(
                                    scale,
                                    "Save all and close checks and saves pending prose and \
                                     project drafts. Close and keep recovery keeps pending edits \
                                     recoverable without saving them to the source files.",
                                    width,
                                )
                            }),
                            crate::localisation::close_details(state),
                            flex_row((
                                text_button(scale, "Keep editing", Writer::keep_editing)
                                    .prop(DefaultFocus(true)),
                                FlexSpacer::Flex(1.),
                                text_button(
                                    scale,
                                    if ordinary_exit {
                                        "Close Writer"
                                    } else if project {
                                        "Close and keep recovery"
                                    } else {
                                        "Close and discard example changes"
                                    },
                                    Writer::close_with_recovery,
                                )
                                .disabled(state.work.busy() || state.project_jobs_busy()),
                                saving.then(|| {
                                    print::primary_action(
                                        scale,
                                        "Save all and close",
                                        &state.palette,
                                        state.work.busy()
                                            || state.project_jobs_busy()
                                            || state.source_repair_pending(),
                                        Writer::save_and_close,
                                    )
                                }),
                            ))
                            .cross_axis_alignment(CrossAxisAlignment::Center)
                            .gap(space::SIBLINGS.px())
                            .width(width.px()),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .gap(space::TEXT.px()),
                    )
                    .padding(space::GROUPS.px())
                    .width((width + 2. * space::GROUPS).px())
                    .prop(ModalSurface(true)),
                ),
            ),))
            .main_axis_alignment(MainAxisAlignment::Start)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .padding(Padding::top(
                ((size::PICKER_TOP - size::ROW) * f64::from(scale)).px(),
            ))
            .dims(Dim::Stretch),
        )
    })
}
