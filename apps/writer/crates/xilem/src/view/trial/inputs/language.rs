use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::space;
use crate::presentation::{Type, Typeset};
use crate::{attention::PaneEntry, trial::Review};
use recite_writer_session::localisation::setup::languages;

pub(super) fn selector(
    state: &Writer,
    disabled: bool,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let settings = &state.trial.settings;
    let scale = state.ui_scale();
    let source_only = state.previews_source_only();
    let entry = state.configuring_trial() && matches!(state.trial.settings.review, Review::Inputs);
    flex_col((
        // Off returns to the language chosen before, or opens the search to choose one; while it
        // is on, no language choice is offered, since none would apply.
        flex_col((styled(
            ControlStyle::Flush,
            checkbox(
                "Use source language only",
                source_only,
                |state: &mut Writer, on| state.set_source_language_only(on),
            )
            .text_size(Type::Body.size(scale))
            .disabled(disabled),
        ),))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .prop(PaneEntry(entry && source_only)),
        (!source_only && !settings.locale.is_empty()).then(|| {
            paragraph(
                scale,
                format!(
                    "Preview language: {}",
                    languages::caption(&settings.locale).unwrap_or_else(|| settings.locale.clone())
                ),
                width,
            )
        }),
        // The list filters as the query is typed.
        (!source_only).then(|| {
            flex_col((
                label(scale, "Find preview language"),
                sized_box(
                    text_input(settings.query.clone(), |state: &mut Writer, value| {
                        state.set_preview_language_query(value)
                    })
                    .typeset(Type::Body, scale)
                    .disabled(disabled)
                    .prop(properties::TextAreaLabel::new("Find preview language")),
                )
                .width(width.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::UNIT.px())
            .prop(PaneEntry(entry))
        }),
        (!source_only && !settings.languages.is_empty()).then(|| choices(state, disabled)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
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
                move |state: &mut Writer| state.choose_preview_language(locale.clone()),
            )
            .disabled(disabled)
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
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
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
