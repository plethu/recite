use super::*;
use recite_writer_session::localisation::catalogue::Draft;

pub(super) fn forms(
    writer: &Writer,
    entry: &Entry,
    draft: &Draft,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let fields = draft
        .forms
        .iter()
        .enumerate()
        .skip(entry.form_page * 4)
        .take(4)
        .map(|(index, value)| form(writer, entry, index, value, blocked))
        .collect::<Vec<_>>();
    flex_col((
        flex_col(fields)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px()),
        (draft.forms.len() > 4).then(|| {
            flex_col((
                label(
                    writer.ui_scale(),
                    format!(
                        "Plural forms {}–{} of {}",
                        entry.form_page * 4 + 1,
                        ((entry.form_page + 1) * 4).min(draft.forms.len()),
                        draft.forms.len()
                    ),
                ),
                text_button(
                    writer.ui_scale(),
                    "Previous plural forms",
                    |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.form_page = entry.form_page.saturating_sub(1);
                        }
                    },
                )
                .disabled(blocked || entry.form_page == 0),
                text_button(
                    writer.ui_scale(),
                    "More plural forms",
                    |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.form_page += 1;
                        }
                    },
                )
                .disabled(blocked || (entry.form_page + 1) * 4 >= draft.forms.len()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}

fn form(
    writer: &Writer,
    entry: &Entry,
    index: usize,
    value: &str,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let key = entry.key.clone();
    let input = text_input(value.to_owned(), move |writer: &mut Writer, value| {
        writer.update_translation(&key, index, value)
    })
    .insert_newline(InsertNewline::OnEnter)
    .font("Literata")
    .text_size(writer.reading_size())
    .disabled(blocked)
    .prop(TextAreaLabel::new(format!(
        "Translation form {}",
        index + 1
    )));
    flex_col((
        label(writer.ui_scale(), entry.details.plural.form_caption(index)),
        sized_box(keyed((entry.key.clone(), index), input))
            .dims((writer.script_width().px(), 180.px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
