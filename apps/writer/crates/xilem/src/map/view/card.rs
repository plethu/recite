//! A normal card button and its explicit move handle compose independent inputs.
use super::*;
use xilem::masonry::properties;
pub(super) fn card(state: &Writer, scene: &Scene, index: usize) -> impl WidgetView<Writer> + use<> {
    let block = &scene.projection.graph.blocks[index];
    let synopsis = &scene.projection.synopsis[index];
    let id = block.id.clone();
    let title = format!(
        "{}{}{}",
        recite_writer_session::choices::display_name(&block.id),
        if block.is_default { " · Entry" } else { "" },
        if scene.projection.graph.endings.contains(&block.id) {
            " · End"
        } else {
            ""
        }
    );
    let selected = scene.selected.as_deref() == Some(&block.id);
    let contents = flex_col((
        label(
            1.,
            format!("{}{title}", if selected { "Selected · " } else { "" }),
        )
        .text_size(18.),
        label(1., synopsis.speaker.clone()),
        paragraph(
            1.,
            synopsis.prose.chars().take(64).collect::<String>(),
            scene.node(index).width as f64 - 32.,
        ),
        (!synopsis.annotations.is_empty()).then(|| label(1., synopsis.annotations.join(" · "))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px());
    let focus_id = id.clone();
    let move_id = id.clone();
    let body = button_any_pointer(contents, move |state: &mut Writer, pointer| {
        state.press_map_beat(id.clone(), pointer);
    })
    .disabled(
        state.phase != crate::Phase::Writing
            || !scene
                .projection
                .is_current(state.session.model().document()),
    )
    .prop(properties::Dimensions::fixed(
        scene.node(index).width.px(),
        (HEIGHT - 28.).px(),
    ))
    .padding(xilem::style::Padding::all(8.px()))
    .background_color(if selected {
        state.palette.selection
    } else {
        state.palette.surface
    });
    let focus = input::input(
        input::Kind::Card,
        false,
        body,
        move |state: &mut Writer, action| state.map_input(Source::Card(focus_id.clone()), action),
    )
    .focus(
        scene
            .focus
            .as_ref()
            .filter(|focus| focus.beat == block.id)
            .and_then(|_| canvas::focus_request(state, scene)),
    );
    let grip = input::input(
        input::Kind::Grip(title),
        state.config.ui.keymap == recite_config::Keymap::Vim,
        label(1., "Move · drag or arrow keys")
            .padding(3.px())
            .background_color(state.palette.inset),
        move |state: &mut Writer, action| state.map_input(Source::Grip(move_id.clone()), action),
    )
    .disabled(state.phase != crate::Phase::Writing || state.navigation.busy() || state.work.busy())
    .prop(properties::Dimensions::fixed(
        scene.node(index).width.px(),
        24.px(),
    ))
    .prop(properties::BorderColor {
        color: state.palette.accent,
    });
    flex_col((focus, grip))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(4.px())
        .dims((scene.node(index).width.px(), HEIGHT.px()))
}
