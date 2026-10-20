use super::*;
use crate::presentation::manuscript_text;
use xilem::core::one_of::Either;

pub(super) fn comparison(writer: &Writer) -> Option<impl WidgetView<Writer> + use<>> {
    let state = &writer.localisation;
    let comparison = state.comparison.as_ref()?;
    let scale = writer.ui_scale();
    let current = state.slot.owner().is_some_and(|owner| {
        owner
            .catalogue
            .comparison_is_current(&comparison.inspection)
    });
    let blocked = writer.project_action_blocked() || state.slot.read_only();
    Some(flex_col((
        label(scale, "Catalogue disk comparison").text_size(20. * scale),
        text_button(scale, "Return to translation queue", |writer: &mut Writer| writer.catalogue_action(work::Action::Query)).disabled(blocked),
        comparison.changes.get(comparison.page).map(|row| flex_col((
            paragraph(scale, format!("{} · part {} of {}", row.caption, comparison.page + 1, comparison.changes.len()), 640.),
            versions(writer, row),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        comparison.changes.is_empty().then(|| paragraph(scale, "There are no translation drafts to compare with the inspected PO file.", 640.)),
        (comparison.changes.len() > 1).then(|| flex_col((
            text_button(scale, "Previous catalogue comparison", |writer: &mut Writer| { if let Some(comparison) = &mut writer.localisation.comparison { comparison.page = comparison.page.saturating_sub(1); } }).disabled(comparison.page == 0),
            text_button(scale, "More catalogue comparison", |writer: &mut Writer| { if let Some(comparison) = &mut writer.localisation.comparison { comparison.page += 1; } }).disabled(comparison.page + 1 >= comparison.changes.len()),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        (!current).then(|| paragraph(scale, "The catalogue changed since this comparison. Inspect it again before choosing a version.", 640.)),
        text_button(scale, "Inspect current catalogue disk version", Writer::compare_catalogue).disabled(blocked),
        text_button(scale, "Keep translation drafts", |writer: &mut Writer| writer.resolve_catalogue(ExternalDecision::KeepDrafts)).disabled(blocked || !current),
        paragraph(scale, "Keeps compatible drafts against the inspected PO file and marks them for review. Save publishes them.", 640.),
        text_button(scale, "Use inspected PO file", |writer: &mut Writer| writer.resolve_catalogue(ExternalDecision::UseDisk)).disabled(blocked || !current),
        paragraph(scale, "Discards current translation drafts and loads the inspected PO file.", 640.),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px()))
}
fn versions(writer: &Writer, row: &ComparisonRow) -> impl WidgetView<Writer> + use<> {
    let available = (writer.viewport.width - 72.).max(240.);
    let wide = available >= 920. * f64::from(writer.ui_scale());
    let width = if wide {
        (available - 12.) / 2.
    } else {
        available
    };
    let before = version(
        writer,
        "Writer translation draft",
        row.before.as_deref(),
        width,
    );
    let after = version(
        writer,
        "Inspected PO translation",
        row.after.as_deref(),
        width,
    );
    if wide {
        Either::A(
            flex_row((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(12.px()),
        )
    } else {
        Either::B(
            flex_col((before, after))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(12.px()),
        )
    }
}
fn version(
    writer: &Writer,
    caption: &'static str,
    value: Option<&str>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    sized_box(
        flex_col((
            label(writer.ui_scale(), caption),
            flex_row((sized_box(manuscript_text(
                writer.reading_size(),
                value.unwrap_or("(No translation in this part)").to_owned(),
            ))
            .width((width - 24.).px()),)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
    .width(width.px())
    .padding(12.px())
    .background_color(writer.palette.inset)
}
