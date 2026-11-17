//! A translation row's margin: its Reviewed toggle, and its details disclosed in place under it.
use super::*;
use crate::presentation::{
    control_style::{ControlStyle, styled},
    disclosure, disclosure_width,
};
use xilem::style::Padding;

/// The longest plural rule the disclosure sets before it ends in an ellipsis.
const RULE_CHARS: usize = 160;

/// The details disclosure and, once a form has words, the Reviewed toggle in the row's margin.
pub(super) fn margin(
    writer: &Writer,
    row: &beat::PassageRow,
    (translation, draft): (&beat::Translation, &Draft),
    blocked: bool,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let scale = writer.ui_scale();
    let reviewed = translation.key.clone();
    let anchor = row.source.id.clone();
    let on = draft.reviewed;
    // Each row's controls name their line, so a beat's rows read apart in a list of controls.
    let line = &row.source.id;
    flex_row((
        // Quiet, so the Reviewed toggle is the row's one filled control.
        styled(
            ControlStyle::Action,
            button(
                disclosure(
                    scale,
                    row.details,
                    writer.palette.ink,
                    label(scale, "Translation details"),
                ),
                move |writer: &mut Writer| writer.toggle_translation_details(&anchor),
            )
            .accessible_label(format!("Translation details · {line}"))
            .expanded(row.details),
        ),
        xilem::view::FlexSpacer::Flex(1.),
        draft
            .forms
            .iter()
            .any(|form| !form.trim().is_empty())
            .then(|| {
                print::text_toggle(
                    scale,
                    (format!("Reviewed translation · {line}"), "Reviewed"),
                    &writer.palette,
                    (draft.reviewed, blocked),
                    move |writer: &mut Writer| writer.review_translation(&reviewed, !on),
                )
            }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
    .width(width.px())
}

/// What the row's entry carries beyond its forms, and the way to its full entry.
pub(super) fn details(
    writer: &Writer,
    row: &beat::PassageRow,
    translation: &beat::Translation,
    blocked: bool,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
    row.details.then(|| {
        let scale = writer.ui_scale();
        let inspect = translation.key.clone();
        let plural = &translation.details.plural;
        let wordings = translation.details.wordings.len();
        flex_col((
            plural.rule().map(|rule| {
                let rule: String = rule.chars().take(RULE_CHARS).collect();
                paragraph(scale, format!("Plural rule: {rule}"), width)
            }),
            (wordings > 1).then(|| {
                paragraph(
                    scale,
                    format!("{wordings} wordings share this source passage."),
                    width,
                )
            }),
            text_button(
                scale,
                "Open translation entry",
                move |writer: &mut Writer| writer.inspect_translation(&inspect),
            )
            .disabled(blocked || writer.project_action_blocked()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px())
        .padding(Padding::left(disclosure_width(scale).px()))
    })
}
