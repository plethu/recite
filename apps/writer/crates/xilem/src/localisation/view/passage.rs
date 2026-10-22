//! Source and translation compose as one retained bilingual manuscript.
mod context;
mod incoming;
mod source;
mod translation;
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
                    .gap(24.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(8.px())
                .boxed(),
                beat::Row::Context(row) => context::context(writer, row, width).boxed(),
            };
            (
                row.identity(&beat.source),
                flex_col((content,)).padding(xilem::style::Padding::from_vh(12.px(), 0.px())),
            )
        })
        .collect();
    Some(
        flex_col((
            print::title(writer.ui_scale(), display_name(&beat.name)),
            print::meta(
                writer.ui_scale(),
                format!(
                    "{} :: {}",
                    writer.session.model().document().key(),
                    beat.name
                ),
            )
            .color(writer.palette.muted),
            actions(writer),
            incoming::incoming(writer, beat),
            paragraph(
                writer.ui_scale(),
                format!(
                    "Source, read only · Translation: {}",
                    entry::catalogue_caption(writer)
                ),
                width,
            ),
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
        .gap(8.px()),
    )
}
fn actions(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    flex_row((
        text_button(writer.ui_scale(), "Edit source", Writer::return_to_writing).disabled(blocked),
        text_button(
            writer.ui_scale(),
            "Open translation queue",
            |writer: &mut Writer| writer.catalogue_action(work::Action::Query),
        )
        .disabled(blocked),
        text_button(
            writer.ui_scale(),
            "Save all translation drafts",
            Writer::save_translation_drafts,
        )
        .disabled(blocked || !writer.localisation.needs_save()),
    ))
    .direction(if width(writer) < 580. * f64::from(writer.ui_scale()) {
        Axis::Vertical
    } else {
        Axis::Horizontal
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .prop(PaneEntry(true))
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
            .gap(8.px()),
        ))
        .gap(8.px())
    })
}
fn width(writer: &Writer) -> f64 {
    (writer.viewport.width - writer.navigation_width() - 32.)
        .max(280.)
        .min(1320. * f64::from(writer.ui_scale()))
}
