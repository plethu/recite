//! Bounded native components inspect the same trace as the running conversation.
use super::*;
use crate::WriterLayout;
use recite_runtime::preview::PreviewTrace;
use recite_writer_session::preview::trace::{
    FIELDS, FieldPreview, ListPage, Place, Projection, RECORDS, RecordPage, Section, ValuePage,
    ValuePosition,
};
use xilem::{masonry::properties::LineBreaking, view::button};
mod record;
mod value;

pub(super) fn panel(writer: &Writer, disabled: bool) -> Option<impl WidgetView<Writer> + use<>> {
    let trace = writer.session.model().preview_trace()?;
    let browser = writer.trial.trace;
    let scale = writer.ui_scale();
    Some(
        flex_col((
            text_button(
                scale,
                if browser.open {
                    "Hide preview trace"
                } else {
                    "Preview trace"
                },
                |writer: &mut Writer| writer.trial.trace.open = !writer.trial.trace.open,
            )
            .disabled(disabled),
            browser.open.then(|| contents(writer, trace, disabled)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(10.px()),
    )
}
fn contents(
    writer: &Writer,
    trace: &PreviewTrace,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let body = match writer.trial.trace.project(trace) {
        Projection::List(page) => list(writer, page, disabled).boxed(),
        Projection::Record(page) => record::record(writer, page, disabled).boxed(),
        Projection::Value(page) => value::value(writer, page, disabled).boxed(),
    };
    flex_col((
        label(writer.ui_scale(), "Runtime trace").text_size(20. * writer.ui_scale()),
        paragraph(writer.ui_scale(), "These records belong to the running preview. Events retain capture order; translation and plural lookups have their own ordered lists.", writer.script_width()),
        sections(writer, disabled),
        body,
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(10.px())
}
fn sections(writer: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let section = writer.trial.trace.section;
    let choices = [Section::Events, Section::Localisation, Section::Plurals]
        .into_iter()
        .map(|next| {
            let caption = format!(
                "Trace: {}{}",
                next.caption(),
                if next == section { " · current" } else { "" }
            );
            text_button(writer.ui_scale(), caption, move |writer: &mut Writer| {
                writer.trial.trace.select(next)
            })
            .disabled(disabled || next == section)
        })
        .collect::<Vec<_>>();
    flex_col(choices)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(6.px())
}
fn list(writer: &Writer, projection: ListPage, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let ListPage {
        section,
        total,
        page,
        pages,
        rows,
    } = projection;
    let scale = writer.ui_scale();
    let rows = rows
        .into_iter()
        .map(|row| {
            let index = row.index;
            flex_row((sized_box(
                button(
                    label(
                        scale,
                        format!("Open trace record {} · {}", index + 1, row.summary),
                    )
                    .prop(LineBreaking::WordWrap),
                    move |writer: &mut Writer| {
                        writer.trial.trace.place = Place::Record { index, page: 0 }
                    },
                )
                .disabled(disabled),
            )
            .width(writer.script_width().px()),))
        })
        .collect::<Vec<_>>();
    flex_col((
        label(
            scale,
            format!(
                "{} · {total} records · page {} of {pages}",
                section.caption(),
                page + 1
            ),
        ),
        (total == 0).then(|| label(scale, "No records in this preview.")),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        (pages > 1).then(|| {
            flex_col((
                text_button(
                    scale,
                    "Previous trace records",
                    move |writer: &mut Writer| {
                        writer.trial.trace.place = Place::List {
                            page: page.saturating_sub(1),
                        }
                    },
                )
                .disabled(disabled || page == 0),
                text_button(scale, "More trace records", move |writer: &mut Writer| {
                    writer.trial.trace.place = Place::List { page: page + 1 }
                })
                .disabled(disabled || page + 1 >= pages),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}

#[cfg(test)]
mod tests;
