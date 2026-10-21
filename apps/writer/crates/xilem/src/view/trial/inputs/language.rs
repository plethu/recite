use super::*;
use recite_writer_session::localisation::setup::languages;

pub(super) fn selector(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let settings = &state.trial.settings;
    let scale = state.ui_scale();
    let caption = if settings.locale.is_empty() {
        "Source language only".into()
    } else {
        format!(
            "Preview language: {}",
            languages::caption(&settings.locale).unwrap_or_else(|| settings.locale.clone())
        )
    };
    flex_col((
        paragraph(scale, caption, state.script_width()),
        text_button(scale, "Use source language only", |state: &mut Writer| {
            state.trial.settings.locale.clear();
            state.trial.settings.languages.clear();
        })
        .disabled(disabled || settings.locale.is_empty()),
        label(scale, "Find preview language"),
        sized_box(
            text_input(settings.query.clone(), |state: &mut Writer, value| {
                state.trial.settings.query = value
            })
            .text_size(16. * scale)
            .disabled(disabled)
            .prop(properties::TextAreaLabel::new("Find preview language")),
        )
        .width(state.script_width().px()),
        text_button(
            scale,
            "Search preview languages",
            Writer::search_preview_languages,
        )
        .disabled(disabled),
        choices(state, disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn choices(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let settings = &state.trial.settings;
    let scale = state.ui_scale();
    let pages = settings.languages.len().div_ceil(8).max(1);
    let page = settings.language_page.min(pages - 1);
    let choices = settings
        .languages
        .iter()
        .skip(page * 8)
        .take(8)
        .map(|choice| {
            let locale = choice.value.clone();
            text_button(
                scale,
                format!("Preview in {} · {locale}", choice.title),
                move |state: &mut Writer| {
                    state.trial.settings.locale = locale.clone();
                    state.trial.settings.languages.clear();
                },
            )
            .disabled(disabled)
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px()),
        (pages > 1).then(|| {
            flex_col((
                label(
                    scale,
                    format!("Preview language page {} of {pages}", page + 1),
                ),
                text_button(
                    scale,
                    "Previous preview languages",
                    move |state: &mut Writer| {
                        state.trial.settings.language_page = page.saturating_sub(1)
                    },
                )
                .disabled(disabled || page == 0),
                text_button(
                    scale,
                    "More preview languages",
                    move |state: &mut Writer| state.trial.settings.language_page = page + 1,
                )
                .disabled(disabled || page + 1 >= pages),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
