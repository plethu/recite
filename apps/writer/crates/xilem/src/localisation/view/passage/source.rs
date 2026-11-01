use super::*;
use crate::presentation::{
    control_style::{ControlStyle, styled},
    manuscript_text,
};
use crate::view::reference;
use recite_writer_model::PassageKind;

pub(super) fn source(
    writer: &Writer,
    row: &beat::PassageRow,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let caption = match &row.source.kind {
        PassageKind::Dialogue { speaker } => speaker
            .as_deref()
            .map(display_name)
            .unwrap_or_else(|| "Narration".into()),
        PassageKind::Choice { .. } => format!("Reply {}", row.reply.unwrap_or(1)),
    };
    let anchor = row.source.id.clone();
    let previous = anchor.clone();
    let next = anchor.clone();
    flex_col((
        print::caps(writer.ui_scale(), caption)
            .color(writer.palette.accent)
            .prop(LineBreaking::WordWrap)
            .width(width.px()),
        manuscript_text(
            prose_size(writer),
            row.pages.get(row.source_page).cloned().unwrap_or_default(),
        )
        .width(width.px()),
        destination(writer, row, width),
        (row.pages.len() > 1).then(|| {
            flex_col((
                label(
                    writer.ui_scale(),
                    format!("Source part {} of {}", row.source_page + 1, row.pages.len()),
                ),
                flex_row((
                    text_button(
                        writer.ui_scale(),
                        "Previous source part",
                        move |writer: &mut Writer| {
                            if let Some(row) = writer.translation_row_mut(&previous) {
                                row.source_page = row.source_page.saturating_sub(1);
                            }
                        },
                    )
                    .disabled(row.source_page == 0),
                    text_button(
                        writer.ui_scale(),
                        "More source passage",
                        move |writer: &mut Writer| {
                            if let Some(row) = writer.translation_row_mut(&next) {
                                row.source_page =
                                    (row.source_page + 1).min(row.pages.len().saturating_sub(1));
                            }
                        },
                    )
                    .disabled(row.source_page + 1 >= row.pages.len()),
                ))
                .gap(8.px()),
            ))
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn destination(
    writer: &Writer,
    row: &beat::PassageRow,
    width: f64,
) -> Option<impl WidgetView<Writer> + use<>> {
    let PassageKind::Choice {
        destination: Some(destination),
    } = &row.source.kind
    else {
        return None;
    };
    let target = destination.clone();
    let terminal = destination == "END";
    Some(
        flex_col((
            terminal.then(|| print::meta(writer.ui_scale(), "◆ END ◆")),
            (!terminal).then(|| {
                styled(
                    ControlStyle::Action,
                    button(
                        label(
                            writer.ui_scale(),
                            format!(
                                "{} {}",
                                if writer.destination_expanded(destination) {
                                    "▾"
                                } else {
                                    "→"
                                },
                                display_name(destination)
                            ),
                        )
                        .color(writer.palette.teal_text)
                        .prop(LineBreaking::WordWrap)
                        .width((width - 24.).max(80.).px()),
                        move |writer: &mut Writer| {
                            writer.toggle_destination_preview(target.clone())
                        },
                    )
                    .disabled(writer.project_action_blocked())
                    .background_color(writer.palette.surface),
                )
            }),
            (!terminal).then(|| reference::destination(writer, destination, width)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
