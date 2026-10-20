//! Bounded, accessible incident-link lists compose below the map canvas.
use super::*;
use crate::map::connections::Direction;

pub(super) fn panel(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), "Connections"),
        connection_list(state, scene, Direction::Outgoing),
        connection_list(state, scene, Direction::Incoming),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
fn connection_list(
    state: &Writer,
    scene: &Scene,
    direction: Direction,
) -> impl WidgetView<Writer> + use<> {
    const PAGE: usize = 4;
    let list = scene.connections.list(direction);
    let indices = &list.indices;
    let page = list.page;
    let heading = direction.label();
    let pages = indices.len().div_ceil(PAGE).max(1);
    let page = page.min(pages - 1);
    let rows = indices
        .iter()
        .skip(page * PAGE)
        .take(PAGE)
        .map(|index| {
            let link = &scene.projection.graph.links[*index];
            let target = direction.target(link);
            let id = target.to_owned();
            sized_box(
                xilem::view::button(
                    label(
                        state.ui_scale(),
                        format!(
                            "{} to {} · {}{}{}",
                            link.origin,
                            link.destination,
                            link.label,
                            if link.conditional {
                                " · conditional"
                            } else {
                                ""
                            },
                            if link.returning { " · return" } else { "" }
                        ),
                    )
                    .prop(properties::LineBreaking::WordWrap),
                    move |state: &mut Writer| state.focus_map_beat(id.clone()),
                )
                .disabled(
                    !scene.projection.graph.ids.contains_key(target)
                        || !scene
                            .projection
                            .is_current(state.session.model().document())
                        || state.phase != crate::Phase::Writing,
                ),
            )
            .width(
                workspace::Layout::resolve(state)
                    .map
                    .min(640. * f64::from(state.ui_scale()))
                    .px(),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        label(state.ui_scale(), format!("{heading} · {}", indices.len())),
        flex_col(rows)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        indices.is_empty().then(|| label(state.ui_scale(), "None")),
        (pages > 1).then(|| {
            flex_row((
                text_button(
                    state.ui_scale(),
                    format!("Previous {heading} connections"),
                    move |state: &mut Writer| {
                        if let Some(scene) = state.map_scene_mut() {
                            scene.connections.list_mut(direction).page = page.saturating_sub(1);
                        }
                    },
                )
                .disabled(page == 0),
                text_button(
                    state.ui_scale(),
                    format!("Next {heading} connections"),
                    move |state: &mut Writer| {
                        if let Some(scene) = state.map_scene_mut() {
                            scene.connections.list_mut(direction).page = (page + 1).min(pages - 1);
                        }
                    },
                )
                .disabled(page + 1 >= pages),
            ))
            .gap(8.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
