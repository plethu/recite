use super::*;
use crate::presentation::source_text;
use recite_writer_session::messages::{MsgId, text as wording};

pub(super) fn guide(
    writer: &Writer,
    entry: &Entry,
    blocked: bool,
) -> Option<impl WidgetView<Writer> + use<>> {
    let guide = &entry.details.plural;
    if !guide.is_plural() {
        return None;
    }
    let scale = writer.ui_scale();
    Some(
        flex_col((
            (!guide.valid()).then(|| {
                paragraph(
                    scale,
                    wording(MsgId::WriterEntryPluralInvalid),
                    entry_width(writer),
                )
            }),
            guide.valid().then(|| {
                text_button(
                    scale,
                    if entry.rule.expanded {
                        "Hide plural rule"
                    } else {
                        "Plural rule"
                    },
                    |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.rule.expanded = !entry.rule.expanded;
                        }
                    },
                )
                .disabled(blocked)
            }),
            (!guide.valid()).then(|| label(scale, "Plural rule").text_size(20. * scale)),
            (entry.rule.expanded || !guide.valid()).then(|| rule(writer, entry, blocked)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}

fn rule(writer: &Writer, entry: &Entry, blocked: bool) -> impl WidgetView<Writer> + use<> {
    flex_col((
        entry.rule.pages.get(entry.rule.page).map(|rule| {
            flex_row((
                sized_box(source_text(14. * writer.ui_scale(), rule.clone()))
                    .width(entry_width(writer).px()),
            ))
        }),
        (entry.rule.pages.len() > 1).then(|| {
            flex_col((
                label(
                    writer.ui_scale(),
                    format!(
                        "Plural rule part {} of {}",
                        entry.rule.page + 1,
                        entry.rule.pages.len()
                    ),
                ),
                text_button(
                    writer.ui_scale(),
                    "Previous plural rule part",
                    |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.rule.page = entry.rule.page.saturating_sub(1);
                        }
                    },
                )
                .disabled(blocked || entry.rule.page == 0),
                text_button(
                    writer.ui_scale(),
                    "More plural rule",
                    |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.rule.page += 1;
                        }
                    },
                )
                .disabled(blocked || entry.rule.page + 1 >= entry.rule.pages.len()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(6.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
