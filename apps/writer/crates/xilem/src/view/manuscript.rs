use super::surface::Surface;
use super::*;
use crate::WriterLayout;
use crate::attention::{Pane, Region};
use crate::presentation::print;
use crate::presentation::space;
use crate::rules;
use recite_writer_model::{EditOperation, StructuralCommand};
mod field;
pub(in crate::view) mod folio;
mod input;
mod insertion;
pub(super) use field::{passage_editor, prose_editor};
use xilem::AnyWidgetView;
use xilem::masonry::kurbo;

pub(super) fn manuscript(state: &Writer, surface: &Surface<'_>) -> impl WidgetView<Writer> + use<> {
    // A readable source with nothing staged needs no repair page.
    let repair = match surface {
        Surface::SourceRepair { error, import }
            if error.is_some() || (*import && state.session.model().has_draft()) =>
        {
            Some((*import, *error))
        }
        _ => None,
    };
    flex_col((
        repair.map(|(import, error)| super::source_repair::panel(state, import, error)),
        content(state, surface),
        // The galley sets these with the selected beat; other surfaces keep them below. A source
        // under repair has nothing to insert into, and its head holds the draft's actions.
        (!galley(state, surface) && repair.is_none())
            .then(|| retained(actions(state), !state.attention.is_focused())),
        diagnostics(state),
        (repair.is_none()
            && !matches!(
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
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
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
        }
        Surface::Reply(reply) => rules::view::panel(state, reply).boxed(),
        Surface::Beat { id, rows } => super::reading::block(state, id, rows).boxed(),
        Surface::Empty => super::reading::empty(state).boxed(),
        // The repair page above says what a broken source needs.
        Surface::SourceRepair { error: Some(_), .. } => flex_col(()).boxed(),
        Surface::SourceRepair { error: None, .. } => paragraph(
            state.ui_scale(),
            "Choose a beat in the outline to read or write it. Use your editor for source editing; \
             source drafts remain recoverable.",
            state.script_width(),
        )
        .boxed(),
    }
}
pub(super) fn draft_actions(
    state: &Writer,
    editing: bool,
    axis: kurbo::Axis,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase() != Phase::Writing || state.session.read_only();
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
    .gap(space::SIBLINGS.px())
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
/// Whether `surface` shows the scene galley, which holds the beat's actions itself.
/// Whether the scene galley shows a selected beat, whose heading carries the beat's actions.
pub(super) fn galley(state: &Writer, surface: &Surface<'_>) -> bool {
    match surface {
        Surface::Manuscript { .. } => {
            state.reading.selected.is_some()
                && state.reading.error.is_none()
                && !state.reading.rows.is_empty()
        }
        Surface::Beat { .. } => true,
        _ => false,
    }
}
pub(super) use insertion::actions;
fn action_axis(state: &Writer) -> kurbo::Axis {
    if state.script_width() < 560. * f64::from(state.ui_scale()) {
        kurbo::Axis::Vertical
    } else {
        kurbo::Axis::Horizontal
    }
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
    .gap(space::SIBLINGS.px())
}
