//! Composed trial settings; only one bounded page of values and languages is mounted.
use super::*;
use recite_writer_session::messages::{MsgId, text};
use xilem::{masonry::properties, view::checkbox};

mod language;
mod values;

pub(super) fn controls(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let settings = &state.trial.settings;
    flex_col((settings.open.then(|| editor(state, disabled)),))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(10.px())
}

fn editor(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let help = paragraph(
        scale,
        "Stage language, wording and values here. Restart captures them; the running conversation keeps its existing inputs.",
        state.script_width(),
    );
    let drafts = checkbox(
        text(MsgId::WriterIncludeDrafts),
        state.trial.settings.include_drafts,
        |state: &mut Writer, value| state.trial.settings.include_drafts = value,
    )
    .text_size(14. * scale)
    .disabled(disabled);
    flex_col((
        label(scale, "Staged preview inputs").text_size(20. * scale),
        help,
        language::selector(state, disabled),
        label(scale, "Preview wording (empty for default)"),
        wording(state, disabled),
        drafts,
        values::values(state, disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(10.px())
}

fn wording(state: &Writer, disabled: bool) -> impl WidgetView<Writer> + use<> {
    sized_box(
        text_input(
            state.trial.settings.variant.clone(),
            |state: &mut Writer, value| {
                state.trial.settings.variant = value;
            },
        )
        .text_size(16. * state.ui_scale())
        .disabled(disabled)
        .prop(properties::TextAreaLabel::new("Preview wording")),
    )
    .width(state.script_width().px())
}

#[cfg(test)]
mod tests;
