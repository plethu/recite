//! Cards remain ordinary native controls; canvas drawing carries only route geometry.
use super::interaction::Source;
use super::*;
use crate::attention::{Pane, Region};
use crate::view::workspace;
use xilem::masonry::properties;
mod canvas;
mod card;
mod cards;
mod connections;
mod overview;
mod toolbar;
use crate::presentation::{label, paragraph, text_button};
use recite_writer_session::map::{layout::HEIGHT, topology};
use toolbar::toolbar;
use xilem::{
    AnyWidgetView, WidgetView,
    masonry::{
        imaging::Painter,
        kurbo::{Affine, Stroke},
        layout::{AsUnit, UnitPoint},
    },
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, button_any_pointer, flex_col, flex_row, resize_observer,
        sized_box, zstack,
    },
};

pub(crate) fn panel(state: &Writer) -> Box<AnyWidgetView<Writer>> {
    let Some(scene) = state.map.scene(state) else {
        return paragraph(
            state.ui_scale(),
            "Choose Map to prepare the accepted scene.",
            700.,
        )
        .boxed();
    };
    let current = scene
        .projection
        .is_current(state.session.model().document());
    flex_col((
        toolbar(state, scene),
        (scene.detail() == Detail::Overview).then(|| {
            label(
                state.ui_scale(),
                "Overview · click a beat or use arrows to read it at full size.",
            )
        }),
        (scene.projection.graph.blocks.len() > 200).then(|| scope_control(state, scene)),
        scene.projection.placement.message().map(|error| {
            flex_col((
                paragraph(
                    state.ui_scale(),
                    format!("Personal placement: {error}. Source is unchanged."),
                    700.,
                ),
                text_button(
                    state.ui_scale(),
                    "Retry personal placement",
                    Writer::refresh_map_layout,
                )
                .disabled(state.navigation.busy() || state.work.busy()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px())
        }),
        (!current).then(|| {
            text_button(state.ui_scale(), "Refresh map", |state: &mut Writer| {
                state.show_workspace(state.map.workspace)
            })
            .disabled(state.navigation.busy())
        }),
        scene.projection.graph.blocks.is_empty().then(|| {
            paragraph(
                state.ui_scale(),
                "No beats yet. Add a beat to start this conversation.",
                700.,
            )
        }),
        canvas::viewport(state, scene).flex(1.),
        connections::panel(state, scene),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
    .prop(Region(Some(Pane::Map)))
    .boxed()
}
fn scope_control(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(
            state.ui_scale(),
            format!(
                "Showing {} of {} beats. The outline and Go to reach every beat.",
                scene
                    .scope
                    .as_ref()
                    .map_or(scene.projection.graph.blocks.len(), |scope| scope
                        .node_indices
                        .len()),
                scene.projection.graph.blocks.len()
            ),
        ),
        text_button(
            state.ui_scale(),
            if scene.whole {
                "Show nearby beats"
            } else {
                "Show whole scene"
            },
            |state: &mut Writer| {
                state.cancel_map_gesture();
                if let Some(scene) = state.map_scene_mut() {
                    scene.whole = !scene.whole;
                    scene.reconcile_selection();
                    scene.camera.fit(scene.viewport, scene.bounds());
                }
            },
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
