use super::*;
use crate::presentation::{manuscript_text, print};
use xilem::masonry::kurbo::Axis;
pub(super) fn queue(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    let width = writer.content_width();
    let wide = width >= 600. * f64::from(scale);
    let measure = if wide { (width - 24.) / 2. } else { width };
    let rows = state
        .queue
        .rows
        .iter()
        .map(|row| {
            let key = state.slot.identity().cloned().map(|catalogue| EntryKey {
                catalogue,
                id: row.id,
            });
            flex_row((
                flex_col((
                    manuscript_text(prose_size(writer), row.source.clone()).width(measure.px()),
                    print::action(
                        scale,
                        format!("Open translation · {}", row.caption),
                        blocked,
                        move |writer: &mut Writer| {
                            if let Some(key) = &key {
                                writer.inspect_translation(key);
                            }
                        },
                    ),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(2.px()),
                flex_col((
                    paragraph(scale, row.status.clone(), measure),
                    (!row.translation.is_empty()).then(|| {
                        manuscript_text(prose_size(writer), row.translation.clone())
                            .width(measure.px())
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px()),
            ))
            .direction(if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(24.px())
            .padding(xilem::style::Padding::from_vh(6.px(), 0.px()))
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_row((
            sized_box(
                text_input(state.query.text.clone(), Writer::set_catalogue_query)
                    .text_size(16. * scale)
                    .prop(TextAreaLabel::new("Search translations")),
            )
            .width(
                (writer.script_width() - 200. * f64::from(scale))
                    .max(180.)
                    .px(),
            ),
            text_button(scale, "Search translations", Writer::search_catalogue)
                .disabled(blocked || !state.has_catalogue()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
        filters(writer, wide),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(10.px()),
        (state.queue.total > 8).then(|| {
            flex_row((
                text_button(scale, "Previous translations", |writer: &mut Writer| {
                    writer.localisation.query.page =
                        writer.localisation.query.page.saturating_sub(1);
                    writer.catalogue_action(work::Action::Query);
                })
                .disabled(blocked || state.queue.page == 0),
                text_button(scale, "More translations", |writer: &mut Writer| {
                    writer.localisation.query.page += 1;
                    writer.catalogue_action(work::Action::Query);
                })
                .disabled(blocked || (state.queue.page + 1) * 8 >= state.queue.total),
            ))
            .gap(8.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}

fn filters(writer: &Writer, wide: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    flex_row((
        radio_group(
            flex_row(
                [
                    ("All entries", Filter::All),
                    ("Needs attention", Filter::Attention),
                    ("Unsaved translations", Filter::Unsaved),
                ]
                .map(|(caption, filter)| {
                    crate::presentation::radio_button(
                        scale,
                        caption,
                        state.query.filter == filter,
                        move |writer: &mut Writer| writer.localisation.query.filter = filter,
                    )
                }),
            )
            .direction(if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        ),
        label(
            scale,
            format!(
                "{} matching entries · page {}",
                state.queue.total,
                state.queue.page + 1
            ),
        )
        .text_size(12. * scale),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
