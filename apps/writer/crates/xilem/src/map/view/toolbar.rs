//! Spatial controls leave the canvas its allocated pane; secondary tools open explicitly.
use super::*;
use crate::presentation::print;
use recite_writer_model::StructuralCommand;
use xilem::view::portal;

pub(super) fn toolbar(state: &Writer, scene: &Scene) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let width = workspace::Layout::resolve(state).map;
    let narrow = width < 600. * f64::from(scale);
    let blocked =
        state.phase != crate::Phase::Writing || state.navigation.busy() || state.work.busy();
    flex_col((
        flex_row((
            print::action(scale, "Open script", blocked, Writer::open_map_beat),
            (!narrow).then(|| zoom(state)),
            (!narrow).then(|| connections(state)),
            print::action(scale, if scene.controls { "Hide map actions" } else { "Map actions" }, blocked, |state: &mut Writer| {
                if let Some(scene) = state.map_scene_mut() { scene.controls = !scene.controls; }
            }),
        )).gap(4.px()),
        scene.controls.then(|| portal(flex_col((
            narrow.then(|| zoom(state)),
            narrow.then(|| connections(state)),
            print::action(scale, "Arrange automatically", blocked, Writer::arrange_map),
            print::action(scale, "Add beat", blocked, |state: &mut Writer| state.navigate(Target::Structure(StructuralCommand::AddBeat))),
            paragraph(scale, "Drag empty space to pan; Ctrl+scroll zooms. Arrows select a beat; Enter opens Script.", width),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())).constrain_horizontal(true).height((120. * f64::from(scale)).px())),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
fn zoom(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_row((
        print::action(scale, "Zoom out", false, |state: &mut Writer| {
            state.zoom_map(1. / 1.2)
        }),
        print::action(scale, "100%", false, Writer::reset_map_zoom),
        print::action(scale, "Zoom in", false, |state: &mut Writer| {
            state.zoom_map(1.2)
        }),
        print::action(scale, "Fit", false, Writer::fit_map),
    ))
    .gap(4.px())
}
fn connections(state: &Writer) -> impl WidgetView<Writer> + use<> {
    print::action(
        state.ui_scale(),
        if state
            .map
            .scene(state)
            .is_some_and(|scene| scene.connections.expanded)
        {
            "Hide connections"
        } else {
            "Show connections"
        },
        false,
        |state: &mut Writer| {
            if let Some(scene) = state.map_scene_mut() {
                scene.connections.expanded = !scene.connections.expanded;
            }
        },
    )
}
