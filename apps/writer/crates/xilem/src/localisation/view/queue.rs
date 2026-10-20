use super::*;
pub(super) fn queue(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    let rows = state
        .queue
        .rows
        .iter()
        .map(|row| {
            let key = state.slot.identity().cloned().map(|catalogue| EntryKey {
                catalogue,
                id: row.id,
            });
            sized_box(
                flex_col((
                    button(
                        label(scale, format!("Open translation · {}", row.caption))
                            .prop(LineBreaking::WordWrap),
                        move |writer: &mut Writer| {
                            if let Some(key) = &key {
                                writer.inspect_translation(key);
                            }
                        },
                    )
                    .disabled(blocked),
                    paragraph(scale, row.source.clone(), 640.),
                    label(scale, row.status.clone()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px()),
            )
            .padding(10.px())
            .background_color(writer.palette.inset)
        })
        .collect::<Vec<_>>();
    flex_col((
        label(scale, "Translation queue").text_size(20. * scale),
        label(scale, "Search translations"),
        text_input(state.query.text.clone(), |writer: &mut Writer, text| {
            writer.localisation.query.text = text
        })
        .text_size(16. * scale)
        .prop(TextAreaLabel::new("Search translations")),
        radio_group(
            flex_col(
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
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        ),
        text_button(scale, "Search translations", Writer::search_catalogue)
            .disabled(blocked || !state.has_catalogue()),
        label(
            scale,
            format!(
                "{} matching entries · page {}",
                state.queue.total,
                state.queue.page + 1
            ),
        ),
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
