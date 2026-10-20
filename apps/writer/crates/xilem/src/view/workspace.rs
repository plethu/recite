//! Stable pane slots change placement without replacing mounted editor state.
use super::*;
use crate::map::Workspace;
use recite_config::{WriterPaneSide, WriterPresentationField};
use xilem::view::{GridExt, GridTrackSize, grid, indexed_stack};

pub(crate) struct Layout {
    pub map: f64,
    pub script: f64,
    stacked: bool,
}
impl Layout {
    pub fn resolve(state: &Writer) -> Self {
        let navigation = if state.attention.is_focused() {
            0.
        } else {
            state.navigation_width() + 20.
        };
        let available = (state.viewport.width - navigation - 48.).max(280.);
        let workspace = state.visible_workspace();
        let preferred = f64::from(
            state
                .config
                .writer
                .presentation
                .value(WriterPresentationField::ScriptWidth),
        );
        let script = preferred.min(available);
        let minimum_map = 320. * f64::from(state.ui_scale());
        let stacked = workspace == Workspace::Split && available < script + minimum_map + 20.;
        let map = if workspace == Workspace::Split && !stacked {
            available - script - 20.
        } else {
            available
        };
        Self {
            map,
            script,
            stacked,
        }
    }
}

pub(super) fn workspace(
    state: &Writer,
    surface: &surface::Surface<'_>,
) -> impl WidgetView<Writer> + use<> {
    let layout = Layout::resolve(state);
    let workspace = state.visible_workspace();
    let map = workspace.has_map();
    let script = workspace != Workspace::Map;
    let map_stack = indexed_stack((
        crate::map::panel(state),
        sized_box(label(1., "")).dims((0.px(), 0.px())),
    ))
    .active(usize::from(!map));
    let script_stack = indexed_stack((
        flex_col((
            manuscript(state, surface),
            retained(trial::panel(state), !state.attention.is_focused()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(20.px()),
        sized_box(label(1., "")).dims((0.px(), 0.px())),
    ))
    .active(usize::from(!script));
    let left = state.config.writer.pane_side == WriterPaneSide::Left;
    let map_slot = if layout.stacked || !left { 0 } else { 2 };
    let script_slot = 2 - map_slot;
    let gap = if map && script { 20. } else { 0. };
    let widths = if layout.stacked {
        [layout.map, 0., 0.]
    } else if left {
        [
            if script { layout.script } else { 0. },
            gap,
            if map { layout.map } else { 0. },
        ]
    } else {
        [
            if map { layout.map } else { 0. },
            gap,
            if script { layout.script } else { 0. },
        ]
    };
    let panes = grid((
        map_stack.grid_pos(if layout.stacked { 0 } else { map_slot }, 0),
        script_stack.grid_pos(
            if layout.stacked { 0 } else { script_slot },
            if layout.stacked { 2 } else { 0 },
        ),
    ))
    .columns(widths.map(|width| GridTrackSize::Fixed(width.px())))
    .rows([
        GridTrackSize::Auto,
        GridTrackSize::Fixed(if layout.stacked { gap.px() } else { 0.px() }),
        GridTrackSize::Auto,
    ]);
    let navigation = indexed_stack((
        navigation(state),
        sized_box(label(1., "")).dims((0.px(), 0.px())),
    ))
    .active(usize::from(state.attention.is_focused()));
    flex_row((navigation, panes))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(if state.attention.is_focused() {
            0.px()
        } else {
            20.px()
        })
}
pub(super) fn workspace_tabs(state: &Writer) -> impl WidgetView<Writer> + use<> {
    indexed_stack((tabs(state), sized_box(label(1., "")).dims((0.px(), 0.px())))).active(
        usize::from(
            state.attention.is_focused()
                || state.workspaces.screen != crate::workspaces::Screen::Write,
        ),
    )
}
fn tabs(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let modes = xilem::view::radio_group(
        flex_row(
            [
                ("Script", Workspace::Script),
                ("Map", Workspace::Map),
                ("Split", Workspace::Split),
            ]
            .map(|(title, workspace)| {
                crate::presentation::radio_button(
                    state.ui_scale(),
                    title,
                    state.map.workspace == workspace,
                    move |state: &mut Writer| state.show_workspace(workspace),
                )
                .disabled(
                    state.phase != Phase::Writing
                        || (workspace != Workspace::Script
                            && (state.navigation.busy() || state.work.busy())),
                )
            }),
        )
        .gap(12.px()),
    );
    flex_row((
        modes,
        state.session.project().map(|_| {
            text_button(state.ui_scale(), "Translations", Writer::open_localisation)
                .disabled(state.project_action_blocked())
        }),
    ))
    .gap(12.px())
}
