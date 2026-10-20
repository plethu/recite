//! Catalogue files, bounded queue rows and entry fields are independent native components.
mod comparison;
mod entry;
mod files;
mod queue;
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
    flex_col((
        label(writer.ui_scale(), "Translations").text_size(28. * writer.ui_scale()),
        (!writer.localisation.has_catalogue()).then(|| paragraph(writer.ui_scale(), "Open a PO catalogue to translate this project. Drafts stay recoverable until you Save.", 640.)),
        files(writer),
        report(writer),
        indexed_stack((queue(writer), flex_col((entry(writer),)), flex_col((comparison::comparison(writer),)))).active(match writer.localisation.destination { Destination::Queue => 0, Destination::Entry => 1, Destination::Comparison => 2 }),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()).prop(Region(Some(Pane::Localisation)))
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
            text_button(scale, "Open unsaved translations", |writer: &mut Writer| {
                writer.phase = Phase::Writing;
                writer.localisation.keep_editing();
                writer.work.keep_editing();
                writer.open_localisation();
                writer.localisation.query.filter = Filter::Unsaved;
                writer.search_catalogue();
            })
            .disabled(writer.project_jobs_busy()),
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
