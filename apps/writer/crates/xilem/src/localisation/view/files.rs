use super::*;
use crate::presentation::space;
use crate::presentation::tokens::size;
use crate::presentation::{Type, Typeset};
use xilem::view::FlexSequence;
/// The queue head's actions: the catalogue files disclosure and Translate current beat, both
/// quiet, and Save all translation drafts as the one Primary while there are drafts to save.
pub(super) fn head_actions(writer: &Writer) -> impl FlexSequence<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let blocked = writer.project_action_blocked();
    let held = blocked || state.slot.read_only();
    (
        print::action(
            scale,
            if state.files_open {
                "Hide catalogue files"
            } else {
                "Catalogue files"
            },
            false,
            |writer: &mut Writer| writer.localisation.files_open = !writer.localisation.files_open,
        ),
        print::action(
            scale,
            "Translate current beat",
            blocked,
            |writer: &mut Writer| writer.open_translation_beat(0),
        ),
        state.needs_save().then(|| {
            print::primary_action(
                scale,
                "Save all translation drafts",
                &writer.palette,
                held,
                Writer::save_translation_drafts,
            )
        }),
    )
}

pub(super) fn files(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let queue = matches!(state.destination, Destination::Queue);
    let blocked = writer.project_action_blocked();
    let held = blocked || state.slot.read_only();
    let catalogue = state.has_catalogue();
    flex_col((
        state.slot.path().filter(|_| !queue).map(|_| {
            paragraph(
                scale,
                writer.localisation.catalogue_caption(),
                writer.content_width(),
            )
        }),
        (queue && state.files_open).then(|| {
            flex_col((
                controls(writer),
                catalogue.then(|| {
                    (
                        text_button(
                            scale,
                            "Compare catalogue disk changes",
                            Writer::compare_catalogue,
                        )
                        .disabled(held),
                        text_button(scale, "Add language", Writer::new_catalogue)
                            .disabled(held || state.dirty()),
                        text_button(
                            scale,
                            "Review source updates",
                            Writer::review_catalogue_sources,
                        )
                        .disabled(held || state.dirty()),
                    )
                }),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        }),
        state.slot.read_only().then(|| {
            paragraph(
                scale,
                "The catalogue is read-only until its operation returns. After an unexpected \
                 stop, close with recovery and reopen before editing translations.",
                size::MEASURE,
            )
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}

fn controls(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    flex_col((
        writer
            .localisation
            .slot
            .path()
            .map(|path| paragraph(scale, path.display().to_string(), size::MEASURE)),
        label(scale, "Catalogue path"),
        text_input(
            writer.localisation.path.clone(),
            |writer: &mut Writer, path| writer.localisation.path = path,
        )
        .typeset(Type::Body, scale)
        .disabled(blocked)
        .prop(TextAreaLabel::new("Catalogue path")),
        flex_col((
            text_button(scale, "Open catalogue", Writer::open_catalogue_path).disabled(
                blocked
                    || writer.localisation.path.trim().is_empty()
                    || writer.localisation.dirty(),
            ),
            // Reload and Close belong to an open catalogue.
            writer.localisation.has_catalogue().then(|| {
                (
                    text_button(scale, "Reload catalogue", |writer: &mut Writer| {
                        writer.catalogue_action(work::Action::Reload)
                    })
                    .disabled(blocked || writer.localisation.dirty()),
                    text_button(scale, "Close catalogue", |writer: &mut Writer| {
                        writer.catalogue_action(work::Action::Close)
                    })
                    .disabled(blocked || writer.localisation.dirty()),
                )
            }),
        ))
        .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
