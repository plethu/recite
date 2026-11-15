//! Catalogue files, bounded queue rows and entry fields are independent native components.
use crate::WriterLayout;
use crate::presentation::material::{Texture, backdrop, screen};
use crate::presentation::tokens::size;
use crate::presentation::{Type, print, space};
use xilem::peniko::Color;
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
        CrossAxisAlignment, button, checkbox, flex_col, flex_row, indexed_stack, sized_box,
        text_input,
    },
};

pub(crate) fn panel(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let compact_entry =
        writer.compact_chrome() && matches!(writer.localisation.destination, Destination::Entry);
    flex_col((
        (!compact_entry && !matches!(writer.localisation.destination, Destination::Passage))
            .then(|| heading(writer)),
        empty(writer).then(|| empty_state(writer)),
        (!compact_entry && !matches!(writer.localisation.destination, Destination::Passage))
            .then(|| files(writer)),
        report(writer),
        destinations(writer),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .prop(Region(Some(Pane::Localisation)))
}
/// The project has no catalogue yet and the queue is showing.
fn empty(writer: &Writer) -> bool {
    !writer.localisation.has_catalogue()
        && matches!(writer.localisation.destination, Destination::Queue)
}
/// No catalogue yet: the heading carries a halftone screen ("not yet fixed"), one sentence says
/// what to do, and the two ways in share one row. Opening a catalogue by path stays behind its
/// action until it is chosen.
fn empty_state(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked() || writer.localisation.slot.read_only();
    flex_col((
        Type::Body.label(
            scale,
            "Add a language to start a catalogue for this project, or open one you already have. \
             Drafts stay recoverable until you save.",
        )
        .color(writer.palette.ink)
        .prop(LineBreaking::WordWrap)
        .width(writer.script_width().px()),
        flex_row((
            print::primary_action(
                scale,
                "Add a language…",
                &writer.palette,
                blocked || writer.localisation.dirty(),
                Writer::new_catalogue,
            ),
            text_button(scale, "Open a catalogue…", |writer: &mut Writer| {
                writer.localisation.files_open = !writer.localisation.files_open;
            })
            .disabled(blocked),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
fn heading(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let queue = matches!(writer.localisation.destination, Destination::Queue);
    crate::view::tool_head(
        writer,
        if empty(writer) {
            "No translations yet"
        } else if queue {
            "Translation queue"
        } else {
            "Translations"
        },
        (
            (queue && writer.localisation.has_catalogue()).then(|| {
                print::meta(writer.ui_scale(), writer.localisation.catalogue_caption())
                    .color(writer.palette.muted)
            }),
            empty(writer).then(|| halftone(writer)),
        ),
    )
}
/// A square of the plum halftone screen beside an empty heading.
fn halftone(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let side = (size::ROW * f64::from(writer.ui_scale())).px();
    backdrop(
        Color::TRANSPARENT,
        screen(Texture::default(), writer.palette.accent),
        flex_col(()).dims((side, side)),
    )
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
                .disabled(writer.phase() != Phase::ConfirmClose || writer.project_jobs_busy() || writer.localisation.slot.read_only() || writer.localisation.saving()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
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
                .gap(space::SIBLINGS.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
