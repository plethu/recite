//! Short windows keep the camera tools visible and disclose secondary controls on demand.
use super::*;
use xilem::masonry::kurbo::Axis;

pub(super) fn toolbar(
    state: &Writer,
    scene: &Scene,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let narrow = workspace::Layout::resolve(state).map < 400. * f64::from(scale);
    flex_row((
        print::action(scale, "Open script", blocked, Writer::open_map_beat),
        zoom(state),
        print::action(
            scale,
            if scene.tools == Tools::Closed {
                "Map tools"
            } else {
                "Hide map tools"
            },
            blocked,
            |state: &mut Writer| {
                if let Some(scene) = state.map_scene_mut() {
                    scene.tools = if scene.tools == Tools::Closed {
                        Tools::Actions
                    } else {
                        Tools::Closed
                    };
                }
            },
        ),
    ))
    .direction(if narrow {
        Axis::Vertical
    } else {
        Axis::Horizontal
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
pub(super) fn actions(
    state: &Writer,
    scene: &Scene,
    width: f64,
    blocked: bool,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_col((
        flex_row((
            print::action(scale, "Add beat", blocked, |state: &mut Writer| {
                state.navigate(Target::Structure(StructuralCommand::AddBeat))
            }),
            print::action(scale, "Arrange", blocked, Writer::arrange_map),
            print::action(scale, "Help", false, |state: &mut Writer| {
                if let Some(scene) = state.map_scene_mut() {
                    scene.tools = Tools::Help;
                }
            }),
        ))
        .gap(4.px()),
        search(state, scene, width.min(320. * f64::from(scale)), blocked),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
