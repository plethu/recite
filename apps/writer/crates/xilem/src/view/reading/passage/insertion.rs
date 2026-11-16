//! Under the passage being written, an insertion affordance adds the same kind of passage after
//! it: "+ Line" after a line, "+ Reply" after a reply. The caret lands in the new passage. A
//! line offers its speaker choice beside it and a reply its destination choice, each as a slip
//! hanging below the row.
use super::*;
use crate::app::keyboard::PickerAnchor;
use recite_writer_model::{RouteOwner, StructuralCommand};

pub(super) fn insertion(
    state: &Writer,
    passage: &Passage,
    grid: &Grid,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let disabled = state.phase() != Phase::Writing
        || state.session.read_only()
        || state.navigation.busy()
        || state.work.busy();
    let title = match passage.kind {
        PassageKind::Dialogue { .. } => ("Insert line after", "Line"),
        PassageKind::Choice { .. } => ("Insert reply after", "Reply"),
    };
    let id = passage.id.clone();
    let line = passage.id.clone();
    let (owner, choice, name) = match &passage.kind {
        PassageKind::Dialogue { speaker } => (
            None,
            "Speaker…",
            speaker.as_deref().map_or_else(
                || "Choose a speaker…".to_owned(),
                |speaker| format!("Change speaker, {speaker}…"),
            ),
        ),
        PassageKind::Choice { .. } => (
            Some(RouteOwner::Reply(passage.id.clone())),
            "Destination…",
            "Change destination…".to_owned(),
        ),
    };
    // The words line up with the passage's text, the plus hanging before them.
    sized_box(
        flex_row((
            print::insert_action(
                scale,
                title,
                &state.palette,
                disabled,
                move |state: &mut Writer| {
                    state.navigate(crate::navigation::Target::Structure(
                        StructuralCommand::InsertAfter(id.clone()),
                    ));
                },
            ),
            styled(
                ControlStyle::Action,
                button(
                    Type::Control.label(scale, choice),
                    move |state: &mut Writer| match &owner {
                        Some(owner) => state.choose_destination(owner.clone()),
                        None => state.choose_speaker(line.clone()),
                    },
                )
                .accessible_label(name)
                .disabled(disabled)
                .prop(PickerAnchor(true)),
            ),
        ))
        .gap(space::SIBLINGS.px()),
    )
    .padding(Padding::left(
        (grid.text() - control_inset(scale) - print::insert_hang(scale))
            .max(0.)
            .px(),
    ))
}
