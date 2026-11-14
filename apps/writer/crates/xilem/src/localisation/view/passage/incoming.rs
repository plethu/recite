use super::*;
use crate::presentation::disclosure;
use xilem::view::button;
pub(super) fn incoming(
    writer: &Writer,
    beat: &beat::Beat,
) -> Option<impl WidgetView<Writer> + use<>> {
    let incoming = &beat.incoming;
    if incoming.links.is_empty() {
        return None;
    }
    let scale = writer.ui_scale();
    let blocked = writer.project_action_blocked();
    let links = incoming
        .links
        .iter()
        .skip(incoming.page * 4)
        .take(4)
        .map(|link| {
            let origin = link.origin.clone();
            button(
                label(
                    scale,
                    format!(
                        "{} → {}",
                        display_name(&origin),
                        link.label.chars().take(160).collect::<String>()
                    ),
                )
                .prop(LineBreaking::WordWrap)
                .width((width(writer) - 24.).max(80.).px()),
                move |writer: &mut Writer| writer.inspect(origin.clone()),
            )
            .disabled(blocked)
        })
        .collect::<Vec<_>>();
    Some(
        flex_col((
            button(
                disclosure(
                    scale,
                    incoming.expanded,
                    writer.palette.ink,
                    label(
                        scale,
                        format!("Incoming prompts ({})", incoming.links.len()),
                    ),
                ),
                |writer: &mut Writer| {
                    if let Some(beat) = &mut writer.localisation.passage {
                        beat.incoming.expanded = !beat.incoming.expanded;
                    }
                },
            ),
            incoming.expanded.then(|| {
                flex_col((
                    flex_col(links)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                        .gap(6.px()),
                    (incoming.links.len() > 4).then(|| {
                        flex_row((
                            text_button(
                                scale,
                                "Previous incoming prompts",
                                |writer: &mut Writer| {
                                    if let Some(beat) = &mut writer.localisation.passage {
                                        beat.incoming.page = beat.incoming.page.saturating_sub(1);
                                    }
                                },
                            )
                            .disabled(incoming.page == 0),
                            text_button(scale, "More incoming prompts", |writer: &mut Writer| {
                                if let Some(beat) = &mut writer.localisation.passage {
                                    beat.incoming.page = (beat.incoming.page + 1)
                                        .min(beat.incoming.links.len().saturating_sub(1) / 4);
                                }
                            })
                            .disabled((incoming.page + 1) * 4 >= incoming.links.len()),
                        ))
                        .gap(8.px())
                    }),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(8.px())
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}
