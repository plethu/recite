use super::*;
use crate::WriterLayout;
use crate::presentation::segments::{segment, segments_along};
use crate::presentation::space;
use crate::presentation::tokens::{breakpoint, size};
use crate::presentation::{Type, Typeset, search_field};
use crate::presentation::{manuscript_text, print};
use xilem::masonry::kurbo::Axis;
pub(super) fn queue(writer: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    let width = writer.content_width();
    let wide = width >= breakpoint::TWO_COLUMNS * f64::from(scale);
    let measure = if wide {
        (width - space::GROUPS) / 2.
    } else {
        width
    };
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
                .gap(space::UNIT.px()),
                flex_col((
                    paragraph(scale, row.status.clone(), measure),
                    (!row.translation.is_empty()).then(|| {
                        manuscript_text(prose_size(writer), row.translation.clone())
                            .width(measure.px())
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(space::SIBLINGS.px()),
            ))
            .direction(if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::GROUPS.px())
            .padding(xilem::style::Padding::from_vh(space::UNIT.px(), 0.px()))
        })
        .collect::<Vec<_>>();
    flex_col((
        // The queue filters as the text is typed; there is no separate Search action.
        search_field(
            scale,
            writer.palette.muted,
            false,
            Some(writer.script_width().max(size::FIELD_MIN)),
            text_input(state.query.text.clone(), Writer::set_catalogue_query)
                .typeset(Type::Body, scale)
                .disabled(!state.has_catalogue())
                .prop(TextAreaLabel::new("Search translations")),
        ),
        filters(writer, wide),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::TEXT.px()),
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
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}

fn filters(writer: &Writer, wide: bool) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let state = &writer.localisation;
    flex_row((
        segments_along(
            &writer.palette,
            if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            },
            [
                ("All entries", Filter::All),
                ("Needs attention", Filter::Attention),
                ("Unsaved translations", Filter::Unsaved),
            ]
            .map(|(caption, filter)| {
                segment(
                    scale,
                    &writer.palette,
                    caption,
                    state.query.filter == filter,
                    false,
                    move |writer: &mut Writer| writer.localisation.query.filter = filter,
                )
            }),
        ),
        label(
            scale,
            format!(
                "{} matching entries · page {}",
                state.queue.total,
                state.queue.page + 1
            ),
        )
        .typeset(Type::Meta, scale),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
