use super::*;
use crate::presentation::manuscript_text;
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
    .text_size(prose_size(writer))
    .disabled(blocked)
    .prop(TranslationField::new(entry.key.clone()))
    .prop(TextAreaLabel::new(format!(
        "Translation form {}",
        index + 1
    )));
    flex_col((
        (!wide(writer)).then(|| {
            let source = entry
                .source
                .get(entry.source_page)
                .map(String::as_str)
                .unwrap_or("");
            let mut excerpt = source.chars().take(180).collect::<String>();
            if source.chars().count() > 180 {
                excerpt.push('…');
            }
            flex_col((
                label(writer.ui_scale(), "Source passage").color(writer.palette.muted),
                manuscript_text(prose_size(writer), excerpt).width(entry_width(writer).px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(4.px())
        }),
        paragraph(
            writer.ui_scale(),
            format!(
                "{} · {}",
                entry.details.plural.form_caption(index),
                writer.localisation.catalogue_caption()
            ),
            entry_width(writer),
        ),
        sized_box(keyed((entry.key.clone(), index), input))
            .dims((entry_width(writer).px(), 180.px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
