use super::*;
use crate::presentation::source_text;

pub(super) fn value(
    writer: &Writer,
    projection: ValuePage,
    disabled: bool,
) -> impl WidgetView<Writer> + use<> {
    let ValuePage {
        position,
        summary,
        value,
    } = projection;
    let scale = writer.ui_scale();
    flex_col((
        text_button(
            scale,
            "Return to trace record",
            move |writer: &mut Writer| {
                writer.trial.trace.place = Place::Record {
                    index: position.record,
                    page: position.field / FIELDS,
                }
            },
        )
        .disabled(disabled),
        paragraph(
            scale,
            format!("Trace record {} · {summary}", position.record + 1),
            writer.script_width(),
        ),
        paragraph(scale, value.name.into_owned(), writer.script_width()),
        label(
            scale,
            format!("Value part {} of {}", value.part + 1, value.parts),
        ),
        flex_row((
            sized_box(source_text(14. * scale, value.value)).width(writer.script_width().px()),
        )),
        (value.parts > 1).then(|| {
            flex_col((
                text_button(
                    scale,
                    "Previous trace value part",
                    move |writer: &mut Writer| {
                        writer.trial.trace.place = Place::Value(ValuePosition {
                            part: position.part.saturating_sub(1),
                            ..position
                        })
                    },
                )
                .disabled(disabled || position.part == 0),
                text_button(scale, "More trace value", move |writer: &mut Writer| {
                    writer.trial.trace.place = Place::Value(ValuePosition {
                        part: position.part + 1,
                        ..position
                    })
                })
                .disabled(disabled || position.part + 1 >= value.parts),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}
