//! Catalogue files, bounded queue rows and entry fields are independent native components.
mod comparison;
mod creation;
mod entry;
mod files;
mod passage;
mod queue;
mod refresh;
mod versions;
use super::*;
use crate::{
    attention::{Pane, Region},
    keyed::keyed,
    presentation::{label, paragraph, text_button},
};
use entry::entry;
use files::files;
use queue::queue;
use xilem::{
    InsertNewline, WidgetView,
    masonry::{
        kurbo::Axis,
        layout::AsUnit,
        properties::{LineBreaking, TextAreaLabel},
    },
    style::Style,
    view::{
        CrossAxisAlignment, button, checkbox, flex_col, flex_row, indexed_stack, radio_group,
        sized_box, text_input,
    },
};

pub(crate) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let compact_entry =
        writer.compact_chrome() && matches!(writer.localisation.destination, Destination::Entry);
    flex_col((
        (!compact_entry && !matches!(writer.localisation.destination, Destination::Passage)).then(|| heading(writer)),
        (!writer.localisation.has_catalogue()).then(|| paragraph(writer.ui_scale(), "Open a PO catalogue or add a language to translate this project. Drafts stay recoverable until you Save.", 640.)),
        (!compact_entry && !matches!(writer.localisation.destination, Destination::Passage)).then(|| files(writer)),
        report(writer),
        destinations(writer),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()).prop(Region(Some(Pane::Localisation)))
}
fn heading(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let queue = matches!(writer.localisation.destination, Destination::Queue);
    let width = writer.content_width();
    let wide = width >= 600. * f64::from(writer.ui_scale());
    flex_row((
        label(
            writer.ui_scale(),
            if queue {
                "Translation queue"
            } else {
                "Translations"
            },
        )
        .text_size(24. * writer.ui_scale())
        .prop(LineBreaking::WordWrap)
        .width(if wide { width * 0.52 } else { width }.px()),
        (queue && writer.localisation.has_catalogue()).then(|| {
            label(writer.ui_scale(), writer.localisation.catalogue_caption())
                .prop(LineBreaking::WordWrap)
                .width(if wide { width * 0.45 } else { width }.px())
                .color(writer.palette.muted)
        }),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(16.px())
}
fn prose_size(writer: &Writer) -> f32 {
    writer.reading_size()
}
fn destinations(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let pages = (
        flex_col((writer.localisation.has_catalogue().then(|| queue(writer)),)),
        flex_col((entry(writer),)),
        flex_col((comparison::comparison(writer),)),
        flex_col((creation::creation(writer),)),
        flex_col((refresh::refresh(writer),)),
        flex_col((passage::passage(writer),)),
    );
    let selected = match writer.localisation.destination {
        Destination::Queue => 0,
        Destination::Entry => 1,
        Destination::Comparison => 2,
        Destination::Create => 3,
        Destination::Refresh => 4,
        Destination::Passage => 5,
    };
    indexed_stack(pages).active(selected)
}
pub(crate) fn close_details(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let owner = writer
        .localisation
        .slot
        .owner()
        .filter(|owner| owner.catalogue.dirty())?;
    let scale = writer.ui_scale();
    let path = writer
        .session
        .project()
        .and_then(|project| owner.catalogue.path.strip_prefix(project.root()).ok())
        .unwrap_or(&owner.catalogue.path);
    let count = owner.catalogue.changed_entries().len();
    Some(
        flex_col((
            paragraph(
                scale,
                format!(
                    "{count} unsaved translation {} · {}",
                    if count == 1 { "draft" } else { "drafts" },
                    path.display()
                ),
                640.,
            ),
            paragraph(
                scale,
                "Reopening this project and catalogue restores these drafts.",
                640.,
            ),
            text_button(scale, "Open unsaved translations", Writer::open_unsaved_translations)
            .disabled(writer.project_jobs_busy()),
            paragraph(scale, "Discarding translations removes all PO drafts. Unsaved source work stays in recovery.", 640.),
            text_button(scale, "Discard translations and close", Writer::discard_translations_and_close)
                .disabled(writer.phase != Phase::ConfirmClose || writer.project_jobs_busy() || writer.localisation.slot.read_only() || writer.localisation.saving()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
fn report(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let report = state.report.get(state.report_page)?;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            paragraph(scale, report.clone(), 640.),
            (state.report.len() > 1).then(|| {
                flex_col((
                    label(
                        scale,
                        format!(
                            "Report part {} of {}",
                            state.report_page + 1,
                            state.report.len()
                        ),
                    ),
                    text_button(
                        scale,
                        "Previous catalogue report part",
                        |writer: &mut Writer| {
                            writer.localisation.report_page =
                                writer.localisation.report_page.saturating_sub(1)
                        },
                    )
                    .disabled(state.report_page == 0),
                    text_button(scale, "More catalogue report", |writer: &mut Writer| {
                        writer.localisation.report_page += 1
                    })
                    .disabled(state.report_page + 1 >= state.report.len()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
