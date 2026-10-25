//! Direct map tools and a bounded search share the compact contextual toolbar.
use super::*;
use crate::presentation::{Icon, print};
use recite_writer_model::StructuralCommand;
use xilem::core::one_of::Either;
use xilem::view::{GridExt, GridParams, GridTrackSize, grid, text_input};
mod compact;

pub(super) fn toolbar(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = workspace::Layout::resolve(state).map;
    let blocked =
        state.phase != crate::Phase::Writing || state.navigation.busy() || state.work.busy();
    let compact = state.viewport.height < 600. * f64::from(scale);
    let controls = if compact {
        Either::A(compact::toolbar(state, scene, blocked))
    } else {
        Either::B(full(state, scene, blocked))
    };
    flex_col((
        controls,
        (compact && scene.tools == Tools::Actions).then(|| compact::actions(state, scene, width, blocked)),
        (scene.tools == Tools::Help).then(|| paragraph(scale,
            "Drag empty space to pan; Ctrl+scroll zooms. Arrows select a beat; Enter opens Script beside Map. Use a card’s move grip to change its personal placement.", width)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
fn full(state: &Writer, scene: &Scene, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = f64::from(state.ui_scale());
    let width = workspace::Layout::resolve(state).map;
    let narrow = width < 600. * scale;
    let search_width = if narrow {
        (width - 200. * scale).clamp(90. * scale, 320. * scale)
    } else {
        (width - 550. * scale).clamp(120. * scale, 320. * scale)
    };
    let tools = tools(state, scene, blocked)
        .grid_params(GridParams::pos(0, 0).with_width(if narrow { 3 } else { 1 }));
    let search =
        search(state, scene, search_width, blocked).grid_pos(u16::from(!narrow), u16::from(narrow));
    let zoom = zoom(state).grid_pos(if narrow { 1 } else { 2 }, u16::from(narrow));
    grid((tools, search, zoom))
        .columns(if narrow {
            [
                GridTrackSize::Fixed(search_width.px()),
                GridTrackSize::Auto,
                GridTrackSize::FRACTION,
                GridTrackSize::Fixed(0.px()),
            ]
        } else {
            [
                GridTrackSize::Auto,
                GridTrackSize::Fixed(search_width.px()),
                GridTrackSize::Auto,
                GridTrackSize::FRACTION,
            ]
        })
        .rows(vec![GridTrackSize::Auto; if narrow { 2 } else { 1 }])
        .gap(12.px())
        .width(width.px())
}
fn tools(state: &Writer, scene: &Scene, blocked: bool) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::action(scale, "Open script", blocked, Writer::open_map_beat),
        print::action(scale, "Add beat", blocked, |state: &mut Writer| {
            state.navigate(Target::Structure(StructuralCommand::AddBeat))
        }),
        print::action(scale, "Arrange", blocked, Writer::arrange_map),
        print::action(
            scale,
            if scene.tools == Tools::Help {
                "Hide help"
            } else {
                "Help"
            },
            false,
            |state: &mut Writer| {
                if let Some(scene) = state.map_scene_mut() {
                    scene.tools = if scene.tools == Tools::Help {
                        Tools::Closed
                    } else {
                        Tools::Help
                    };
                }
            },
        ),
    ))
    .gap(4.px())
}
fn search(
    state: &Writer,
    scene: &Scene,
    width: f64,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    text_input(scene.search.clone(), Writer::find_map_beat)
        .text_size(14. * state.ui_scale())
        .placeholder("Find a beat…")
        .disabled(blocked)
        .prop(properties::TextAreaLabel::new("Find a beat"))
        .width(width.px())
}
fn zoom(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::icon_action(
            scale,
            "Zoom out",
            Icon::Minus,
            &state.palette,
            false,
            |state: &mut Writer| state.zoom_map(1. / 1.2),
        ),
        print::action(
            scale,
            format!(
                "{:.0}%",
                state.map.scene(state).map_or(1., |scene| scene.camera.zoom) * 100.
            ),
            false,
            Writer::reset_map_zoom,
        ),
        print::icon_action(
            scale,
            "Zoom in",
            Icon::Plus,
            &state.palette,
            false,
            |state: &mut Writer| state.zoom_map(1.2),
        ),
        print::action(scale, "Fit", false, Writer::fit_map),
    ))
    .gap(4.px())
    .border_color(state.palette.rule)
    .border_width(1.px())
    .corner_radius(3.px())
}
