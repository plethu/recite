//! Stable pane slots change placement without replacing mounted editor state.
use super::*;
use crate::map::Workspace;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::group::named;
use recite_config::{WriterPaneSide, WriterPresentationField};
use xilem::masonry::{layout::Dim, properties::CheckmarkSize};
use xilem::view::{GridExt, GridTrackSize, grid, indexed_stack, portal};

pub(crate) struct Layout {
    pub map: f64,
    pub script: f64,
    script_pane: f64,
    compact_split: bool,
    available: f64,
}
impl Layout {
    pub fn resolve(state: &Writer) -> Self {
        let available = state.content_width();
        let workspace = state.visible_workspace();
        let preferred = f64::from(
            state
                .config
                .writer
                .presentation
                .value(WriterPresentationField::ScriptWidth),
        );
        let scale = f64::from(state.ui_scale());
        let minimum_map = 320. * scale;
        let compact_split =
            workspace == Workspace::Split && available < preferred + minimum_map + 20.;
        let full_script = workspace == Workspace::Script || compact_split;
        let script = (preferred + if full_script { 144. } else { 0. }).min(available);
        let map = if workspace == Workspace::Split && !compact_split {
            available - script - 20.
        } else {
            available
        };
        Self {
            map,
            script,
            script_pane: if full_script { available } else { script },
            compact_split,
            available,
        }
    }
}

pub(super) fn workspace(
    state: &Writer,
    surface: &surface::Surface<'_>,
) -> impl WidgetView<Writer> + use<> {
    let layout = Layout::resolve(state);
    let workspace = state.visible_workspace();
    let map = workspace.has_map() && !layout.compact_split;
    let script = workspace != Workspace::Map;
    let map_stack = indexed_stack((
        crate::map::panel(state),
        flex_col(()).dims((0.px(), 0.px())),
    ))
    .active(usize::from(!map));
    let script_stack = indexed_stack((
        named(
            "Script pane",
            portal(
                flex_row((manuscript(state, surface),))
                    .main_axis_alignment(xilem::view::MainAxisAlignment::Center)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .width(layout.script_pane.px()),
            )
            .constrain_horizontal(true),
        ),
        flex_col(()).dims((0.px(), 0.px())),
    ))
    .active(usize::from(!script));
    let left = state.config.writer.pane_side == WriterPaneSide::Left;
    let map_slot = if left { 2 } else { 0 };
    let script_slot = 2 - map_slot;
    let gap = if map && script { 20. } else { 0. };
    let widths = if left {
        [
            if script { layout.script_pane } else { 0. },
            gap,
            if map { layout.map } else { 0. },
        ]
    } else {
        [
            if map { layout.map } else { 0. },
            gap,
            if script { layout.script_pane } else { 0. },
        ]
    };
    let panes = grid((
        map_stack.grid_pos(map_slot, 0),
        script_stack.grid_pos(script_slot, 0),
    ))
    .columns(widths.map(|width| GridTrackSize::Fixed(width.px())))
    .rows([GridTrackSize::FRACTION]);
    let preview = state.trial.visible && !state.attention.is_focused();
    flex_col((
        retained(panes, !preview).flex(if preview { 0. } else { 1. }),
        retained(trial::panel(state), preview).flex(if preview { 1. } else { 0. }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(0.px())
    .width(layout.available.px())
}
pub(super) fn workspace_tabs(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let modes = xilem::view::radio_group(
        flex_row(
            [
                ("Script", Workspace::Script),
                ("Map", Workspace::Map),
                ("Split", Workspace::Split),
            ]
            .map(|(title, workspace)| {
                styled(
                    ControlStyle::Segment,
                    crate::presentation::radio_button(
                        state.ui_scale(),
                        title,
                        state.map.workspace == workspace,
                        move |state: &mut Writer| state.show_workspace(workspace),
                    )
                    .label_color(if state.map.workspace == workspace {
                        state.palette.on_stamp
                    } else {
                        state.palette.ink
                    })
                    .disabled(
                        state.phase != Phase::Writing
                            || (workspace != Workspace::Script
                                && (state.navigation.busy() || state.work.busy())),
                    )
                    .prop(CheckmarkSize {
                        side: 0.px(),
                        gap: 0.px(),
                    })
                    .background_color(if state.map.workspace == workspace {
                        state.palette.stamp
                    } else {
                        state.palette.surface
                    }),
                )
            }),
        )
        .gap(2.px()),
    );
    flex_row((
        sized_box(modes)
            .width(Dim::MaxContent)
            .padding(2.px())
            .border_color(state.palette.muted)
            .border_width(1.px()),
        flex_col(()).flex(1.),
    ))
    .gap(0.px())
}

pub(super) fn authoring_modes(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let localise = state.workspaces.screen == crate::workspaces::Screen::Localisation;
    sized_box(xilem::view::radio_group(
        flex_row(
            [("Write", false), ("Localize", true)].map(|(title, selected)| {
                let active = selected == localise;
                styled(
                    ControlStyle::Segment,
                    crate::presentation::radio_button(
                        state.ui_scale(),
                        title,
                        active,
                        move |state: &mut Writer| {
                            if selected {
                                state.open_localisation();
                            } else {
                                state.return_to_writing();
                            }
                        },
                    )
                    .disabled(
                        state.project_action_blocked()
                            || (selected && state.session.project().is_none()),
                    )
                    .label_color(if active {
                        state.palette.on_stamp
                    } else {
                        state.palette.ink
                    })
                    .prop(CheckmarkSize {
                        side: 0.px(),
                        gap: 0.px(),
                    })
                    .background_color(if active {
                        state.palette.stamp
                    } else {
                        state.palette.surface
                    }),
                )
            }),
        )
        .gap(2.px()),
    ))
    .padding(2.px())
    .border_color(state.palette.muted)
    .border_width(1.px())
}
