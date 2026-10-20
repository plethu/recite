use super::surface::Surface;
use super::*;
use crate::attention::{Pane, Region};
use crate::rules;
use recite_writer_model::{EditOperation, StructuralCommand};
use xilem::AnyWidgetView;
use xilem::masonry::kurbo;
use xilem::masonry::properties;

pub(super) fn manuscript(state: &Writer, surface: &Surface<'_>) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), surface.title()),
        retained(actions(state), !state.attention.is_focused()),
        retained(
            flex_col((destination(state, surface),)),
            !state.attention.is_focused(),
        ),
        content(state, surface),
        diagnostics(state),
        (!matches!(surface, Surface::Reply(_)))
            .then(|| draft_actions(state, matches!(surface, Surface::Manuscript { .. }))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
    .prop(Region(Some(Pane::Script)))
}
fn content(state: &Writer, surface: &Surface<'_>) -> Box<AnyWidgetView<Writer>> {
    match surface {
        Surface::Manuscript { passage, .. } => prose_editor(state, passage).boxed(),
        Surface::Reply(reply) => rules::view::panel(state, reply).boxed(),
        Surface::Beat { id, rows } => super::reading::block(state, id, rows).boxed(),
        Surface::Empty => super::reading::empty(state).boxed(),
        Surface::SourceRepair { error: Some(error), .. } => super::reading::repair(state, error).boxed(),
        Surface::SourceRepair { error: None, .. } =>
            paragraph(state.ui_scale(), "Choose a beat in the outline to read or write it. Use your editor for source editing; source drafts remain recoverable.", state.script_width()).boxed(),
    }
}
fn prose_editor(state: &Writer, passage: &str) -> impl WidgetView<Writer> + use<> {
    sized_box(keyed(
        (
            state.session.model().document().key().clone(),
            passage.to_owned(),
        ),
        text_input(
            state.session.model().draft().to_owned(),
            |state: &mut Writer, text| {
                state.change(|model| {
                    model.set_draft(text);
                    Ok(())
                });
            },
        )
        .insert_newline(InsertNewline::OnEnter)
        .font("Literata")
        .text_size(state.reading_size())
        .placeholder("Manuscript")
        .disabled(state.phase != Phase::Writing)
        .prop(properties::TextAreaLabel::new("Manuscript"))
        .prop(properties::PlaceholderTextSize {
            size: state.reading_size(),
        })
        .prop(Manuscript(true)),
    ))
    .dims((
        state.script_width().px(),
        if state.trial.visible {
            100.px()
        } else {
            260.px()
        },
    ))
}
fn destination(state: &Writer, surface: &Surface<'_>) -> Option<impl WidgetView<Writer> + use<>> {
    let Surface::Manuscript {
        route: Some(owner), ..
    } = surface
    else {
        return None;
    };
    let owner = owner.clone();
    Some(
        text_button(
            state.ui_scale(),
            "Change reply destination…",
            move |state: &mut Writer| state.choose_destination(owner.clone()),
        )
        .disabled(state.phase != Phase::Writing || state.navigation.busy() || state.work.busy()),
    )
}
fn draft_actions(state: &Writer, editing: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
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
        edit_button(state, "Undo applied", EditOperation::Undo, !closing),
        edit_button(state, "Redo applied", EditOperation::Redo, !closing),
    ))
    .direction(action_axis(state))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn edit_button(
    state: &Writer,
    title: &'static str,
    operation: EditOperation,
    enabled: bool,
) -> impl WidgetView<Writer> + use<> {
    text_button(state.ui_scale(), title, move |state: &mut Writer| {
        state.navigate(crate::navigation::Target::Edit(operation))
    })
    .disabled(!enabled)
}
fn actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase != Phase::Writing || state.navigation.busy() || state.work.busy();
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
        text_button(state.ui_scale(), "Read beat", Writer::read_current)
            .disabled(closing || !selected),
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
    text_button(state.ui_scale(), title, move |state: &mut Writer| {
        state.navigate(crate::navigation::Target::Structure(command.clone()))
    })
    .disabled(!enabled)
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
