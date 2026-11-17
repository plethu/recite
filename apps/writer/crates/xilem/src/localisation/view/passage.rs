//! Source and translation compose as one retained bilingual manuscript.
use crate::WriterLayout;
use crate::presentation::space;
mod context;
mod incoming;
mod source;
mod translation;
mod updates;
use super::*;
use crate::{attention::PaneEntry, keyed::column, presentation::print};
use recite_writer_session::choices::display_name;
use xilem::{AnyWidgetView, masonry::kurbo::Axis};

pub(super) fn passage(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let beat = writer
        .localisation
        .passage
        .as_ref()
        .filter(|beat| beat.is_current(writer.session.model()))?;
    let width = width(writer);
    let wide = width >= 600. * f64::from(writer.ui_scale());
    let measure = if wide { (width - 24.) / 2. } else { width };
    let rows = beat
        .rows
        .iter()
        .map(|row| {
            let content: Box<AnyWidgetView<Writer>> = match row {
                beat::Row::Passage(row) => flex_col((
                    (!row.groups.is_empty()).then(|| {
                        paragraph(
                            writer.ui_scale(),
                            format!("Within {}", row.groups.join(" → ")),
                            width,
                        )
                    }),
                    flex_row((
                        source::source(writer, row, measure),
                        translation::translation(writer, row, measure),
                    ))
                    .direction(if wide {
                        Axis::Horizontal
                    } else {
                        Axis::Vertical
                    })
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::GROUPS.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px())
                .boxed(),
                beat::Row::Context(row) => context::context(writer, row, width).boxed(),
            };
            (
                row.identity(&beat.source),
                flex_col((content,))
                    .padding(xilem::style::Padding::from_vh(space::KINDS.px(), 0.px())),
            )
        })
        .collect();
    Some(
        flex_col((
            crate::view::tool_head(writer, display_name(&beat.name), actions(writer))
                .prop(PaneEntry(true)),
            running_head(writer),
            updates::updates(writer, beat, width),
            incoming::incoming(writer, beat),
            (!writer.localisation.has_catalogue()).then(|| {
                paragraph(
                    writer.ui_scale(),
                    "Choose a catalogue in the translation queue to edit alongside this source.",
                    width,
                )
            }),
            (beat.total == 0).then(|| {
                paragraph(
                    writer.ui_scale(),
                    "This beat has no dialogue or source context yet.",
                    width,
                )
            }),
            column(rows),
            pages(writer, beat),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
/// The sheet's own actions: back to the source, the queue, and Save all drafts, the sheet's one
/// primary, shown only while drafts wait, as in the queue.
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    let drafts = writer.localisation.needs_save();
    flex_row((
        text_button(scale, "Edit source", Writer::return_to_writing).disabled(blocked),
        text_button(scale, "Open translation queue", |writer: &mut Writer| {
            writer.catalogue_action(work::Action::Query)
        })
        .disabled(blocked),
        drafts.then(|| {
            print::primary_action(
                scale,
                "Save all translation drafts",
                &writer.palette,
                blocked,
                Writer::save_translation_drafts,
            )
        }),
    ))
    .gap(space::SIBLINGS.px())
}
/// The running head under the title: the source file, then the catalogue's language and file.
fn running_head(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let source = writer.session.model().document().key().to_string();
    print::meta(
        writer.ui_scale(),
        if writer.localisation.has_catalogue() {
            format!("{source} · {}", writer.localisation.catalogue_caption())
        } else {
            source
        },
    )
    .color(writer.palette.muted)
}
fn pages(writer: &Writer, beat: &beat::Beat) -> Option<impl WidgetView<Writer> + use<>> {
    (beat.total > beat::PAGE).then(|| {
        let page = beat.page;
        let blocked = writer.project_action_blocked();
        flex_col((
            label(
                writer.ui_scale(),
                format!(
                    "Beat records {}–{} of {}",
                    page * beat::PAGE + 1,
                    ((page + 1) * beat::PAGE).min(beat.total),
                    beat.total
                ),
            ),
            flex_row((
                text_button(
                    writer.ui_scale(),
                    "Previous bilingual passages",
                    move |writer: &mut Writer| writer.open_translation_beat(page.saturating_sub(1)),
                )
                .disabled(blocked || page == 0),
                text_button(
                    writer.ui_scale(),
                    "More bilingual passages",
                    move |writer: &mut Writer| writer.open_translation_beat(page + 1),
                )
                .disabled(blocked || (page + 1) * beat::PAGE >= beat.total),
            ))
            .gap(space::SIBLINGS.px()),
        ))
        .gap(space::SIBLINGS.px())
    })
}
fn width(writer: &Writer) -> f64 {
    writer
        .page_width()
        .min(1320. * f64::from(writer.ui_scale()))
}
