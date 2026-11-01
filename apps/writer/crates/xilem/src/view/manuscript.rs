use super::surface::Surface;
use super::*;
use crate::attention::{Pane, Region};
use crate::presentation::print;
use crate::rules;
use recite_writer_model::{EditOperation, StructuralCommand};
mod field;
mod folio;
mod input;
pub(super) use field::{passage_editor, prose_editor};
use xilem::AnyWidgetView;
use xilem::masonry::kurbo;

pub(super) fn manuscript(state: &Writer, surface: &Surface<'_>) -> impl WidgetView<Writer> + use<> {
    flex_col((
        matches!(surface, Surface::SourceRepair { import: true, .. })
            .then(|| super::source_repair::panel(state)),
        content(state, surface),
        retained(actions(state), !state.attention.is_focused()),
        diagnostics(state),
        (!matches!(
            surface,
            Surface::Reply(_) | Surface::Beat { .. } | Surface::Manuscript { .. }
        ))
        .then(|| {
            draft_actions(
                state,
                matches!(surface, Surface::Manuscript { .. }),
                action_axis(state),
            )
        }),
        folio::footer(state),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Script)))
}
fn content(state: &Writer, surface: &Surface<'_>) -> Box<AnyWidgetView<Writer>> {
    match surface {
        Surface::Manuscript { passage, .. } => {
            if let Some(id) = state.reading.selected.as_deref()
                && state.reading.error.is_none()
                && !state.reading.rows.is_empty()
            {
                super::reading::block(state, id, &state.reading.rows).boxed()
            } else {
                prose_editor(state, passage, state.script_width()).boxed()
            }
        },
        Surface::Reply(reply) => rules::view::panel(state, reply).boxed(),
        Surface::Beat { id, rows } => super::reading::block(state, id, rows).boxed(),
        Surface::Empty => super::reading::empty(state).boxed(),
        Surface::SourceRepair { error: Some(error), .. } => super::reading::repair(state, error).boxed(),
        Surface::SourceRepair { error: None, .. } =>
            paragraph(state.ui_scale(), "Choose a beat in the outline to read or write it. Use your editor for source editing; source drafts remain recoverable.", state.script_width()).boxed(),
    }
}
pub(super) fn draft_actions(
    state: &Writer,
    editing: bool,
    axis: kurbo::Axis,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing || state.session.read_only();
    let has_draft = state.session.model().has_draft();
    flex_row((
        edit_button(
            state,
            "Apply draft",
            EditOperation::Apply,
            !closing && !state.navigation.busy() && editing && has_draft,
        ),
        text_button(scale, "Discard draft", |state: &mut Writer| {
            state.change(|model| {
                model.discard();
                Ok(())
            })
        })
        .disabled(closing || !has_draft),
    ))
    .direction(axis)
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn edit_button(
    state: &Writer,
    title: &'static str,
    operation: EditOperation,
    enabled: bool,
) -> impl WidgetView<Writer> + use<> {
    text_button(
        state.ui_scale(),
        title,
        move |state: &mut Writer| match operation {
            EditOperation::Undo => state.undo_redo(false),
            EditOperation::Redo => state.undo_redo(true),
            _ => state.navigate(crate::navigation::Target::Edit(operation)),
        },
    )
    .disabled(!enabled)
}
fn actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase != Phase::Writing
        || state.session.read_only()
        || state.navigation.busy()
        || state.work.busy();
    let repair = state.reading.script().is_err();
    let selected = state.reading.selected.is_some() && state.reading.error.is_none() && !repair;
    flex_row((
        add_button(
            state,
            "Add beat",
            StructuralCommand::AddBeat,
            !closing && !repair,
        ),
        add_button(
            state,
            "Add line",
            StructuralCommand::AddLine,
            !closing && selected,
        ),
        add_button(
            state,
            "Add reply",
            StructuralCommand::AddReply,
            !closing && selected,
        ),
    ))
    .direction(action_axis(state))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn action_axis(state: &Writer) -> kurbo::Axis {
    if state.script_width() < 560. * f64::from(state.ui_scale()) {
        kurbo::Axis::Vertical
    } else {
        kurbo::Axis::Horizontal
    }
}
fn add_button(
    state: &Writer,
    title: &'static str,
    command: StructuralCommand,
    enabled: bool,
) -> impl WidgetView<Writer> + use<> {
    print::action(
        state.ui_scale(),
        title,
        !enabled,
        move |state: &mut Writer| {
            state.navigate(crate::navigation::Target::Structure(command.clone()))
        },
    )
}
fn diagnostics(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let diagnostics = state.session.model().document().diagnostic_snapshot();
    flex_col(
        (0..diagnostics.len().min(3))
            .filter_map(|index| diagnostics.get(index))
            .map(|diagnostic| {
                paragraph(
                    state.ui_scale(),
                    format!(
                        "Source diagnostics · line {} · {}",
                        diagnostic.span.start.line(),
                        diagnostic.message
                    ),
                    state.script_width(),
                )
            })
            .collect::<Vec<_>>(),
    )
    .gap(6.px())
}
