use super::*;

pub(super) fn wordings(
    writer: &Writer,
    entry: &Entry,
    blocked: bool,
) -> Option<impl WidgetView<Writer> + use<>> {
    let choices = &entry.details.wordings;
    if choices.len() < 2 {
        return None;
    }
    let scale = writer.ui_scale();
    let pages = choices.len().div_ceil(8);
    let page = entry.wording_page.min(pages - 1);
    let choices = choices
        .iter()
        .skip(page * 8)
        .take(8)
        .map(|wording| {
            let key = EntryKey {
                catalogue: entry.key.catalogue.clone(),
                id: wording.entry,
            };
            let current = wording.entry == entry.key.id;
            let caption: String = wording.caption.chars().take(160).collect();
            let caption = format!(
                "Wording: {caption}{}",
                if current { " · current" } else { "" }
            );
            let action = button(
                label(scale, caption).prop(LineBreaking::WordWrap),
                move |writer: &mut Writer| writer.inspect_translation(&key),
            )
            .disabled(
                blocked
                    || writer.project_action_blocked()
                    || writer.localisation.saving()
                    || current,
            );
            flex_row((sized_box(action).width(writer.script_width().px()),))
        })
        .collect::<Vec<_>>();
    Some(
        flex_col((
            label(scale, "Wording").text_size(20. * scale),
            flex_col(choices)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px()),
            (pages > 1).then(|| {
                flex_col((
                    label(scale, format!("Wording page {} of {pages}", page + 1)),
                    text_button(scale, "Previous wordings", move |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.wording_page = page.saturating_sub(1);
                        }
                    })
                    .disabled(blocked || page == 0),
                    text_button(scale, "More wordings", move |writer: &mut Writer| {
                        if let Some(entry) = &mut writer.localisation.entry {
                            entry.wording_page = page + 1;
                        }
                    })
                    .disabled(blocked || page + 1 >= pages),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(6.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}

#[cfg(test)]
mod tests;
