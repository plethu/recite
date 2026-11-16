//! Under the passage being written, an insertion affordance adds the same kind of passage after
//! it: "+ Line" after a line, "+ Reply" after a reply. The caret lands in the new passage.
use super::*;
use recite_writer_model::StructuralCommand;

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
    // The words line up with the passage's text, the plus hanging before them.
    sized_box(print::insert_action(
        scale,
        title,
        &state.palette,
        disabled,
        move |state: &mut Writer| {
            state.navigate(crate::navigation::Target::Structure(
                StructuralCommand::InsertAfter(id.clone()),
            ));
        },
    ))
    .padding(Padding::left(
        (grid.text() - control_inset(scale) - print::insert_hang(scale))
            .max(0.)
            .px(),
    ))
}
