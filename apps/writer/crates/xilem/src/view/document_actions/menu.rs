//! A bounded action panel leaves the writing workspace allocated and retained.
use super::*;
use crate::attention::{DocumentMenu, Pane, Region};
use crate::presentation::{group::named, print};
use xilem::{
    core::{MessageResult, map_message_result},
    masonry::kurbo::Axis,
    view::portal,
};

pub(in crate::view) fn panel(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = (600. * f64::from(scale))
        .min(state.viewport.width - 48.)
        .max(280.);
    sized_box(named(
        "Document actions menu",
        flex_col((
            flex_row((
                label(scale, "Document actions")
                    .text_size(18. * scale)
                    .flex(1.),
                print::action(
                    scale,
                    "Dismiss document actions",
                    false,
                    |state: &mut Writer| state.document_menu = DocumentMenu::Dismissed,
                ),
            ))
            .gap(8.px()),
            map_message_result(
                flex_col((
                    portal(content(state, width - 26.))
                        .constrain_horizontal(true)
                        .flex(1.),
                    footer(state),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(12.px()),
                |state: &mut Writer, result: MessageResult<()>| {
                    if matches!(result, MessageResult::Action(())) {
                        state.document_menu = DocumentMenu::Closed;
                    }
                    result
                },
            )
            .flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(12.px())
        .prop(Region(Some(Pane::DocumentActions))),
    ))
    .dims((width.px(), (state.viewport.height * 0.65).px()))
    .padding(12.px())
    .background_color(state.palette.inset)
    .border_color(state.palette.accent)
    .border_width(1.px())
}

fn action_axis(state: &Writer, width: f64) -> Axis {
    if width < 560. * f64::from(state.ui_scale()) {
        Axis::Vertical
    } else {
        Axis::Horizontal
    }
}

fn content(state: &Writer, width: f64) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    let axis = action_axis(state, width);
    flex_col((
        state.reading.selected_passage().map(|passage| {
            flex_col((
                label(scale, format!("Passage · {}", passage.label)),
                manuscript::draft_actions(state, true, axis),
                matches!(
                    passage.kind,
                    recite_writer_model::PassageKind::Choice { .. }
                )
                .then(|| reading::reply_actions(state, &passage.id)),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px())
        }),
        flex_row((
            text_button(scale, "Redo applied", |state: &mut Writer| {
                state.undo_redo(true)
            })
            .disabled(closing || state.navigation.busy() || state.work.busy()),
            text_button(scale, "Undo applied", |state: &mut Writer| {
                state.undo_redo(false)
            })
            .disabled(closing || state.navigation.busy() || state.work.busy()),
            text_button(scale, "Configure preview", Writer::configure_trial).disabled(closing),
        ))
        .direction(axis)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
        flex_row((
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
        .direction(axis)
        .cross_axis_alignment(CrossAxisAlignment::Start)
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
        ))
        .direction(axis)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn footer(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    flex_row((
        state.session.project().is_some().then(|| {
            text_button(scale, "Save file", Writer::save).disabled(
                closing
                    || state.work.busy()
                    || state.localisation.saving()
                    || state.navigation.busy()
                    || state.source_repair_pending(),
            )
        }),
        state.session.project().is_some().then(|| {
            text_button(scale, "Save all", Writer::save_all).disabled(
                closing
                    || state.work.busy()
                    || state.localisation.saving()
                    || state.localisation.working()
                    || state.navigation.busy()
                    || state.source_repair_pending(),
            )
        }),
        text_button(scale, "Focus writing", Writer::toggle_focus)
            .disabled(closing || state.work.busy() || state.navigation.busy()),
        text_button(scale, "Close", Writer::request_close).disabled(closing),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
