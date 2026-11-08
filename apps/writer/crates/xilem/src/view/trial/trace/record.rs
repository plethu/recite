use super::*;
use crate::WriterLayout;
use crate::presentation::source_text;

pub(super) fn record(
    writer: &Writer,
    projection: RecordPage,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let RecordPage {
        index,
        summary,
        page,
        pages,
        fields,
    } = projection;
    let scale = writer.ui_scale();
    let fields = fields
        .into_iter()
        .map(|preview| field(writer, preview, index, disabled))
        .collect::<Vec<_>>();
    flex_col((
        text_button(
            scale,
            "Return to trace records",
            move |writer: &mut Writer| {
                writer.trial.trace.place = Place::List {
                    page: index / RECORDS,
                }
            },
        )
        .disabled(disabled),
        paragraph(
            scale,
            format!("Trace record {} · {summary}", index + 1),
            writer.script_width(),
        ),
        flex_col(fields)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px()),
        (pages > 1).then(|| {
            flex_col((
                label(
                    scale,
                    format!("Record details · page {} of {pages}", page + 1),
                ),
                text_button(
                    scale,
                    "Previous record details",
                    move |writer: &mut Writer| {
                        writer.trial.trace.place = Place::Record {
                            index,
                            page: page.saturating_sub(1),
                        }
                    },
                )
                .disabled(disabled || page == 0),
                text_button(scale, "More record details", move |writer: &mut Writer| {
                    writer.trial.trace.place = Place::Record {
                        index,
                        page: page + 1,
                    }
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
fn field(
    writer: &Writer,
    preview: FieldPreview,
    record: usize,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let caption = format!("Read complete trace value · {}", preview.name);
    let field = preview.index;
    let value = if preview.truncated {
        format!("{}…", preview.value)
    } else {
        preview.value
    };
    flex_col((
        paragraph(scale, preview.name, writer.script_width()),
        flex_row((sized_box(source_text(14. * scale, value)).width(writer.script_width().px()),)),
        preview.truncated.then(|| {
            flex_row((sized_box(
                button(
                    label(scale, caption).prop(LineBreaking::WordWrap),
                    move |writer: &mut Writer| {
                        writer.trial.trace.place = Place::Value(ValuePosition {
                            record,
                            field,
                            part: 0,
                        })
                    },
                )
                .disabled(disabled),
            )
            .width(writer.script_width().px()),))
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
