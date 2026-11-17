//! The insertion affordances after the beat's last row. "+ Line" and "+ Reply" add after the
//! passage being written when it is of their kind, so a passage can be inserted between rows, or
//! else at the beat's end; the caret lands in the new passage. The row sits after the beat, so
//! choosing a passage never moves another.
use super::*;
use crate::presentation::{Page, control_inset};
use recite_writer_model::{PassageKind, View};

pub(in crate::view) fn actions(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase() != Phase::Writing
        || state.session.read_only()
        || state.navigation.busy()
        || state.work.busy();
    let repair = state.reading.script().is_err();
    let selected = state.reading.selected.is_some() && state.reading.error.is_none() && !repair;
    let active = active(state);
    let after = |reply: bool| {
        active
            .as_ref()
            .filter(|(_, kind)| matches!(kind, PassageKind::Choice { .. }) == reply)
            .map(|(id, _)| StructuralCommand::InsertAfter(id.clone()))
    };
    flex_row((
        add_button(
            state,
            ("Add line", "Line"),
            after(false).unwrap_or(StructuralCommand::AddLine),
            !closing && selected,
        ),
        add_button(
            state,
            ("Add reply", "Reply"),
            after(true).unwrap_or(StructuralCommand::AddReply),
            !closing && selected,
        ),
        // A new beat joins the end of the scene.
        add_button(
            state,
            ("Add beat", "Beat"),
            StructuralCommand::AddBeat,
            !closing && !repair,
        ),
    ))
    .direction(action_axis(state))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
    // The words line up with the manuscript's text column, the plus hanging before it.
    .padding(Padding::left(
        if state.reading.rows.is_empty() {
            0.
        } else {
            (super::super::reading::text_column(state)
                - control_inset(Page::of(state).control())
                - print::insert_hang(Page::of(state).control()))
            .max(0.)
        }
        .px(),
    ))
}

/// The passage being written in the selected beat, with its kind.
fn active(state: &Writer) -> Option<(String, PassageKind)> {
    let View::Passage(id) = state.session.model().view() else {
        return None;
    };
    state
        .reading
        .rows
        .iter()
        .find_map(|row| match &row.content {
            crate::reading::Content::Passage { passage, .. } if &passage.id == id => {
                Some((passage.id.clone(), passage.kind.clone()))
            }
            _ => None,
        })
}

/// An insertion affordance: "+ Line" adds a line, named "Add line".
fn add_button(
    state: &Writer,
    title: (&'static str, &'static str),
    command: StructuralCommand,
    enabled: bool,
) -> impl WidgetView<Writer> + use<> {
    // The row zooms with the sheet it ends, never below the interface's own size.
    print::insert_action(
        Page::of(state).control(),
        title,
        &state.palette,
        !enabled,
        move |state: &mut Writer| {
            state.navigate(crate::navigation::Target::Structure(command.clone()))
        },
    )
}
