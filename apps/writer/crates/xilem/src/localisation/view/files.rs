use super::*;
use crate::attention::PaneEntry;
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::presentation::{Type, Typeset};
use xilem::masonry::kurbo::Axis;
pub(super) fn files(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let queue = matches!(state.destination, Destination::Queue);
    let blocked = writer.project_action_blocked();
    let held = blocked || state.slot.read_only();
    let catalogue = state.has_catalogue();
    let narrow = writer.viewport.width - writer.navigation_width()
        < breakpoint::TWO_COLUMNS * f64::from(scale);
    flex_col((
        state.slot.path().filter(|_| !queue).map(|_| {
            paragraph(
                scale,
                writer.localisation.catalogue_caption(),
                writer.content_width(),
            )
        }),
        (queue && catalogue).then(|| {
            flex_row((
                text_button(
                    scale,
                    if state.files_open {
                        "Hide catalogue files"
                    } else {
                        "Catalogue files"
                    },
                    |writer: &mut Writer| {
                        writer.localisation.files_open = !writer.localisation.files_open
                    },
                ),
                text_button(scale, "Translate current beat", |writer: &mut Writer| {
                    writer.open_translation_beat(0)
                })
                .disabled(blocked),
                text_button(
                    scale,
                    "Save all translation drafts",
                    Writer::save_translation_drafts,
                )
                .disabled(held || !state.needs_save()),
            ))
            .direction(if narrow {
                Axis::Vertical
            } else {
                Axis::Horizontal
            })
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
            .prop(PaneEntry(true))
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
