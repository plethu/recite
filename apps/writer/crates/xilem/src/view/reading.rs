use super::*;
use crate::reading::Content;
use recite_writer_model::PassageKind;
use xilem::style::Padding;

pub(super) fn inspection(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let error = state
        .reading
        .script()
        .err()
        .or(state.reading.error.as_deref())
        .map(str::to_owned);
    if let Some(error) = error {
        let repair = "Choose Open in editor to repair the source, then Reload (keep recovery copy). Your source bytes are preserved.";
        Either::A(
            flex_col((
                label(scale, "Source editing in your editor"),
                sized_box(prose(scale, repair)).width(state.script_width().px()),
                sized_box(prose(scale, error)).width(state.script_width().px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        )
    } else if state
        .reading
        .script()
        .is_ok_and(<[recite_writer_model::ScriptBlock]>::is_empty)
    {
        Either::B(Either::A(
            flex_col((
                label(scale, "No beats yet").text_size(22. * scale),
                prose(
                    scale,
                    "Add a beat to start writing. Comments and existing source are preserved.",
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        ))
    } else {
        Either::B(Either::B(
            if matches!(state.session.model().view(), View::Block(_)) {
                Either::A(block(state))
            } else {
                Either::B(sized_box(prose(scale, "Choose a beat in the outline to read or write it. Use your editor for source editing; source drafts remain recoverable.")).width(state.script_width().px()))
            },
        ))
    }
}

fn block(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    const PAGE: usize = 16;
    let pages = state.reading.rows.len().div_ceil(PAGE).max(1);
    state.reading.page = state.reading.page.min(pages - 1);
    let closing = state.phase != Phase::Writing;
    let rows = state
        .reading
        .rows
        .iter()
        .enumerate()
        .skip(state.reading.page * PAGE)
        .take(PAGE)
        .map(|(index, row)| {
            let inset = row.groups.len().min(3) as f64 * 24.;
            let width = (state.script_width() - inset).px();
            let content = match &row.content {
                Content::Passage(passage) => {
                    let id = passage.id.clone();
                    let caption = match &passage.kind {
                        PassageKind::Dialogue { speaker } => {
                            speaker.clone().unwrap_or_else(|| "Narration".into())
                        }
                        PassageKind::Choice { destination } => format!(
                            "Reply → {}",
                            destination.as_deref().unwrap_or("Choose a destination")
                        ),
                    };
                    Either::A(
                        flex_col((
                            label(scale, caption),
                            flex_row((sized_box(crate::presentation::manuscript_text(
                                state.reading_size(),
                                passage.text.clone(),
                            ))
                            .width(width),)),
                            text_button(
                                scale,
                                format!("Edit passage {}", index + 1),
                                move |state: &mut Writer| state.select(id.clone()),
                            )
                            .disabled(closing),
                        ))
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .gap(8.px()),
                    )
                }
                Content::Heading(text) => Either::B(Either::A(
                    sized_box(prose(scale, text.clone()).text_size(state.reading_size()))
                        .width(width),
                )),
                Content::Note(text) => Either::B(Either::B(
                    sized_box(prose(scale, text.clone())).width(width),
                )),
            };
            flex_col((
                (!row.groups.is_empty()).then(|| {
                    sized_box(
                        prose(scale, format!("Within {}", row.groups.join(" → "))).text_size(14.),
                    )
                    .width(width)
                }),
                content,
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
            .padding(Padding::left(inset.px()))
        })
        .collect::<Vec<_>>();
    flex_col((
        label(
            scale,
            format!("Beat · {}", state.reading.selected.as_deref().unwrap_or("")),
        )
        .text_size(22. * scale),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(20.px()),
        state.reading.rows.is_empty().then(|| {
            prose(
                scale,
                "No dialogue yet. Add a line or reply, or edit the source in your editor.",
            )
        }),
        (pages > 1).then(|| {
            flex_row((
                text_button(scale, "Previous beat entries", |state: &mut Writer| {
                    state.reading.page = state.reading.page.saturating_sub(1)
                })
                .disabled(closing || state.reading.page == 0),
                label(scale, format!("{} of {pages}", state.reading.page + 1)),
                text_button(scale, "Next beat entries", |state: &mut Writer| {
                    state.reading.page += 1
                })
                .disabled(closing || state.reading.page + 1 >= pages),
            ))
            .gap(8.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(16.px())
}
