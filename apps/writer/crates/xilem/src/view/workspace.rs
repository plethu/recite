//! Stable pane slots change placement without replacing mounted editor state.
use super::*;
use crate::map::Workspace;
use crate::presentation::group::named;
use crate::presentation::segments::{segment, segments};
use recite_config::{WriterPaneSide, WriterPresentationField};
use xilem::view::{GridExt, GridTrackSize, grid, indexed_stack, portal};
mod sheet;

pub(crate) struct Layout {
    pub map: f64,
    pub script: f64,
    script_pane: f64,
    pub compact_split: bool,
    pub split_available: bool,
    /// Whether Script reads as a bordered sheet on the desk rather than full-bleed paper.
    pub sheet: bool,
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
        // Prose has its own reading size, but the surrounding controls still scale.
        let minimum_script = preferred.max(400. * scale);
        let split_available = available >= minimum_script + minimum_map + 20.;
        let compact_split = workspace == Workspace::Split && !split_available;
        let full_script = workspace == Workspace::Script || compact_split;
        let script = if full_script {
            preferred + 144.
        } else {
            minimum_script
        }
        .min(available);
        let map = if workspace == Workspace::Split && !compact_split {
            available - script - 20.
        } else {
            available
        };
        let script_pane = if full_script { available } else { script };
        let sheet = workspace == Workspace::Script
            && state.viewport.width >= sheet::MINIMUM_WINDOW
            && script + sheet::EXTENT <= available;
        Self {
            map,
            // Full-bleed paper keeps a margin of paper either side of the measure.
            script: if sheet {
                script
            } else {
                script.min(script_pane - 2. * sheet::BLEED)
            },
            script_pane,
            compact_split,
            split_available,
            sheet,
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
        flex_col((
            super::reading::page_controls(state, surface),
            named(
                "Script pane",
                super::reading::viewport::follow_reading(
                    state,
                    portal(sheet::paper(state, &layout, manuscript(state, surface)))
                        .constrain_horizontal(true)
                        .background_color(sheet::ground(state, &layout))
                        .prop(ReadingViewport::capture(state)),
                ),
            )
            .flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(8.px()),
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
    let split_available = Layout::resolve(state).split_available;
    let modes = segments(
        &state.palette,
        [
            ("Script", Workspace::Script),
            ("Map", Workspace::Map),
            // Disabled when the window is too narrow; the status line gives the reason.
            ("Split", Workspace::Split),
        ]
        .map(|(title, workspace)| {
            segment(
                state.ui_scale(),
                &state.palette,
                title,
                state.map.workspace == workspace,
                state.phase() != Phase::Writing
                    || (workspace == Workspace::Split && !split_available)
                    || (workspace != Workspace::Script
                        && (state.navigation.busy() || state.work.busy())),
                move |state: &mut Writer| state.show_workspace(workspace),
            )
        }),
    );
    flex_row((modes, flex_col(()).flex(1.))).gap(0.px())
}
