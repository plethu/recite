use super::{Phase, Writer, keyed::keyed};
use crate::presentation::{label, paragraph, prose, text_button};
use recite_writer_model::{View, Workbench};
use xilem::core::one_of::Either;
use xilem::masonry::layout::AsUnit;
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, flex_col, flex_row, portal, sized_box, text_input};
use xilem::{InsertNewline, WidgetView};

mod manuscript;
#[derive(Clone, Copy, Default, PartialEq)]
pub(super) struct Manuscript(pub bool);
impl xilem::masonry::core::UsesProperty<Manuscript> for xilem::masonry::widgets::TextInput {}
impl xilem::masonry::core::Property for Manuscript {
    fn static_default() -> &'static Self {
        static OTHER: Manuscript = Manuscript(false);
        &OTHER
    }
}
mod navigation;
mod reading;
use manuscript::manuscript;
mod trial;
use navigation::navigation;

pub(super) fn writer(state: &mut Writer) -> impl WidgetView<Writer, Widget: Sized> + use<> {
    let scale = state.ui_scale();
    xilem::core::fork(
        portal(
            sized_box(
                flex_col((
                    header(state),
                    paragraph(
                        scale,
                        state.status.clone(),
                        state.script_width() + state.navigation_width() + 20.,
                    ),
                    state.external.path().map(|path| {
                        let path = state
                            .session
                            .project()
                            .and_then(|project| path.strip_prefix(project.root()).ok())
                            .unwrap_or(path);
                        paragraph(
                            scale,
                            format!(
                                "Editor request active for {}. You can keep writing.",
                                path.display()
                            ),
                            state.script_width() + state.navigation_width() + 20.,
                        )
                    }),
                    recovery_copy(state),
                    close_prompt(state),
                    flex_row((
                        navigation(state),
                        flex_col((manuscript(state), trial::panel(state)))
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .gap(20.px()),
                    ))
                    .gap(20.px()),
                    state
                        .session
                        .project()
                        .and_then(|project| {
                            project
                                .sessions()
                                .find_map(|session| session.recovery_error())
                        })
                        .map(|error| {
                            label(
                                scale,
                                format!("Recovery failed: {error}. Save or keep the window open."),
                            )
                        }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(16.px()),
            )
            .padding(24.px())
            .background_color(state.palette.canvas),
        )
        .constrain_horizontal(true),
        (
            crate::trial::worker(),
            crate::editor_work::worker(),
            crate::external_editor::worker(),
        ),
    )
}

fn header(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    flex_col((
        label(scale, "Recite Writer").text_size(28. * scale),
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
                .disabled(closing || state.work.busy())
            }),
            state.session.project().is_some().then(|| {
                text_button(scale, "Save all", |state: &mut Writer| {
                    state.save_all();
                })
                .disabled(closing || state.work.busy())
            }),
            state.session.project().is_some().then(|| {
                text_button(scale, "Open in editor", Writer::open_editor)
                    .disabled(closing || state.work.busy() || state.external.busy())
            }),
            (state.session.project().is_some() && state.editor.is_some()).then(|| {
                text_button(scale, "Use system default", Writer::open_system_editor)
                    .disabled(closing || state.work.busy() || state.external.busy())
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
                    .disabled(closing || state.work.busy())
            }),
            text_button(scale, "Close", Writer::request_close).disabled(closing),
        ))
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn recovery_copy(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
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

fn diagnostics(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    flex_col(
        (0..diagnostics.len().min(3))
            .filter_map(|index| diagnostics.get(index))
            .map(|diagnostic| {
                sized_box(prose(
                    scale,
                    format!(
                        "Source diagnostics · line {} · {}",
                        diagnostic.span.start.line(),
                        diagnostic.message
                    ),
                ))
                .width(state.script_width().px())
            })
            .collect::<Vec<_>>(),
    )
    .gap(6.px())
}

fn close_prompt(state: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
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
                            .disabled(state.work.busy())
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
