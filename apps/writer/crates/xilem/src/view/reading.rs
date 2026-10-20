use super::*;
use crate::reading::Content;
use recite_writer_model::PassageKind;
use xilem::style::Padding;

pub(super) fn inspection(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let error = state
        .reading
        .script()
        .err()
        .or(state.reading.error.as_deref())
        .map(str::to_owned);
    if let Some(error) = error {
        let repair = if state.editor.is_some() {
            "Choose Open in editor to repair the source, then Reload (keep recovery copy). Your source bytes are preserved."
        } else {
            "Open this file in your preferred editor, then choose Reload (keep recovery copy). Your source bytes are preserved."
        };
        Either::A(
            flex_col((
                label("Source editing in your editor"),
                sized_box(prose(repair)).width(560.px()),
                sized_box(prose(error)).width(560.px()),
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
                label("No beats yet").text_size(22.),
                prose("Add a beat to start writing. Comments and existing source are preserved."),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        ))
    } else {
        Either::B(Either::B(
            if matches!(state.session.model().view(), View::Block(_)) {
                Either::A(block(state))
            } else {
                Either::B(sized_box(prose("Choose a beat in the outline to read or write it. Use your editor for source editing; source drafts remain recoverable.")).width(560.px()))
            },
        ))
    }
}

fn block(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
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
            let width = (560. - inset).px();
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
                            label(caption),
                            sized_box(prose(passage.text.clone()).text_size(20.)).width(width),
                            text_button(
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
                    sized_box(prose(text.clone()).text_size(20.)).width(width),
                )),
                Content::Note(text) => {
                    Either::B(Either::B(sized_box(prose(text.clone())).width(width)))
                }
            };
            flex_col((
                (!row.groups.is_empty()).then(|| {
                    sized_box(prose(format!("Within {}", row.groups.join(" → "))).text_size(14.))
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
        label(format!(
            "Beat · {}",
            state.reading.selected.as_deref().unwrap_or("")
        ))
        .text_size(22.),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(20.px()),
        state.reading.rows.is_empty().then(|| {
            prose("No dialogue yet. Add a line or reply, or edit the source in your editor.")
        }),
        (pages > 1).then(|| {
            flex_row((
                text_button("Previous beat entries", |state: &mut Writer| {
                    state.reading.page = state.reading.page.saturating_sub(1)
                })
                .disabled(closing || state.reading.page == 0),
                label(format!("{} of {pages}", state.reading.page + 1)),
                text_button("Next beat entries", |state: &mut Writer| {
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
