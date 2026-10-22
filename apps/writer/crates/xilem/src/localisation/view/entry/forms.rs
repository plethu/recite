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
    .text_size(prose_size(writer))
    .disabled(blocked)
    .prop(TranslationField::new(entry.key.clone()))
    .prop(TextAreaLabel::new(format!(
        "Translation form {}",
        index + 1
    )));
    flex_col((
        paragraph(
            writer.ui_scale(),
            format!(
                "{} · {}",
                entry.details.plural.form_caption(index),
                catalogue_caption(writer)
            ),
            writer.script_width(),
        ),
        sized_box(keyed((entry.key.clone(), index), input))
            .dims((writer.script_width().px(), 180.px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
}
pub(in crate::localisation::view) fn catalogue_caption(writer: &Writer) -> String {
    let catalogue = &writer.localisation;
    let filename = catalogue
        .catalogue_path()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "PO catalogue".into());
    let language = catalogue.slot.document().and_then(|document| {
        document
            .headers()
            .iter()
            .find(|header| header.key().eq_ignore_ascii_case("Language"))
            .map(|header| header.value())
    });
    match language.filter(|language| !language.is_empty()) {
        Some(language) => format!(
            "{} · {}",
            language.chars().take(80).collect::<String>(),
            filename.chars().take(80).collect::<String>()
        ),
        None => filename.chars().take(80).collect(),
    }
}
