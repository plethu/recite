//! Every field targets one catalogue lifetime, PO entry and plural arm.
use super::*;
use crate::presentation::space;
use recite_writer_session::localisation::catalogue::Draft;
mod review;

pub(super) fn translation(
    writer: &Writer,
    row: &beat::PassageRow,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let current = row
        .translation
        .as_ref()
        .filter(|translation| {
            writer.localisation.slot.identity() == Some(&translation.key.catalogue)
        })
        .and_then(|translation| {
            writer
                .localisation
                .slot
                .draft(translation.key.id)
                .map(|draft| (translation, draft))
        });
    flex_col((
        current.as_ref().map(|(translation, draft)| fields(writer, row, translation, draft, width)),
        current.is_none().then(|| paragraph(writer.ui_scale(), if writer.localisation.has_catalogue() {
            "No matching entry for this source wording; the banner above reviews source updates."
        } else { "No catalogue selected." }, width)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(space::SIBLINGS.px())
}
fn fields(
    writer: &Writer,
    row: &beat::PassageRow,
    translation: &beat::Translation,
    draft: &Draft,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let key = &translation.key;
    let blocked = writer.localisation.slot.read_only() || writer.phase() != Phase::Writing;
    let status = writer.localisation.slot.owner().map_or_else(
        || "Catalogue action in progress".into(),
        |owner| TranslationStatus::entry_label(&owner.catalogue, key.id),
    );
    let forms = draft
        .forms
        .iter()
        .enumerate()
        .skip(row.form_page * 4)
        .take(4)
        .map(|(arm, value)| {
            let update = key.clone();
            let height = field_height(value, prose_size(writer), width);
            let input = text_input(value.clone(), move |writer: &mut Writer, value| {
                writer.update_translation(&update, arm, value)
            })
            .insert_newline(InsertNewline::OnEnter)
            .font("Literata")
            .text_size(prose_size(writer))
            .disabled(blocked)
            .prop(TranslationField::new(key.clone()))
            .prop(TextAreaLabel::new(format!(
                "Translation · passage {} · form {}",
                row.number.ordinal(),
                arm + 1
            )));
            (
                arm,
                flex_col((
                    (draft.forms.len() > 1).then(|| {
                        paragraph(
                            writer.ui_scale(),
                            translation.details.plural.form_caption(arm),
                            width,
                        )
                    }),
                    sized_box(keyed((key.clone(), arm), input)).dims((width.px(), height.px())),
                ))
                .gap(space::SIBLINGS.px()),
            )
        })
        .collect();
    let save = key.clone();
    let discard = key.clone();
    let changed = writer
        .localisation
        .slot
        .owner()
        .is_some_and(|owner| owner.catalogue.changed(key.id));
    flex_col((
        paragraph(writer.ui_scale(), status, width),
        column(forms),
        form_pages(writer, row, draft, blocked),
        review::margin(writer, row, (translation, draft), blocked, width),
        review::details(writer, row, translation, blocked, width),
        changed.then(|| {
            flex_row((
                text_button(
                    writer.ui_scale(),
                    "Save translation",
                    move |writer: &mut Writer| writer.save_translation_key(&save),
                )
                .disabled(
                    blocked || writer.project_jobs_busy() || !translation.details.plural.valid(),
                ),
                text_button(
                    writer.ui_scale(),
                    "Discard draft",
                    move |writer: &mut Writer| writer.discard_translation(&discard),
                )
                .disabled(blocked || writer.localisation.saving()),
            ))
            .direction(if width < 300. * f64::from(writer.ui_scale()) {
                Axis::Vertical
            } else {
                Axis::Horizontal
            })
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
fn form_pages(
    writer: &Writer,
    row: &beat::PassageRow,
    draft: &Draft,
    blocked: bool,
) -> Option<impl WidgetView<Writer> + use<>> {
    (draft.forms.len() > 4).then(|| {
        let previous = row.source.id.clone();
        let next = previous.clone();
        let last_page = draft.forms.len().saturating_sub(1) / 4;
        flex_row((
            text_button(
                writer.ui_scale(),
                "Previous plural forms",
                move |writer: &mut Writer| {
                    if let Some(row) = writer.translation_row_mut(&previous) {
                        row.form_page = row.form_page.saturating_sub(1);
                    }
                },
            )
            .disabled(blocked || row.form_page == 0),
            text_button(
                writer.ui_scale(),
                "More plural forms",
                move |writer: &mut Writer| {
                    if let Some(row) = writer.translation_row_mut(&next) {
                        row.form_page = (row.form_page + 1).min(last_page);
                    }
                },
            )
            .disabled(blocked || (row.form_page + 1) * 4 >= draft.forms.len()),
        ))
        .gap(space::SIBLINGS.px())
    })
}

fn field_height(value: &str, font: f32, width: f64) -> f64 {
    let line = f64::from(font) * 1.4;
    let columns = (width / (f64::from(font) * 0.55)).max(1.) as usize;
    let lines = value
        .lines()
        .count()
        .max(value.chars().count().div_ceil(columns));
    (lines as f64 * line + 24.).clamp(line + 24., (line + 24.).max(300.))
}
