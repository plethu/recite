//! Script is a sheet resting on the grained desk, or full-bleed paper where none fits. The sheet
//! is the frame: it stays put and the manuscript scrolls within it, so the sheet always reaches
//! the pane's foot and its running head and foot stay in place.
use super::super::manuscript::folio;
use super::*;
use crate::presentation::grid::Grid;
use crate::presentation::space;
use xilem::masonry::layout::UnitPoint;
use xilem::masonry::{
    layout::Dim,
    properties::{BoxShadow, Dimensions},
    widgets::Flex,
};
use xilem::peniko::Color;
use xilem::style::Padding;
use xilem::view::MainAxisAlignment;
use xilem::view::zstack;

/// The hard offset of the sheet's shadow, right and down.
const SHADOW: f64 = 3.;

/// The manuscript's scrolled content: the columns, padded to the grid. Both forms keep one widget
/// tree, so switching between them never replaces the mounted manuscript.
pub(super) fn columns<V: WidgetView<Writer>>(
    grid: &Grid,
    manuscript: V,
) -> impl WidgetView<Writer, Widget = Flex> + use<V> {
    flex_col((manuscript,))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .padding(Padding {
            top: if grid.sheet {
                space::REGIONS
            } else {
                space::KINDS
            }
            .px(),
            bottom: space::REGIONS.px(),
            left: grid.pad.px(),
            right: 0.px(),
        })
}

/// Script's paper, with the galley's running head and foot.
pub(super) fn frame<V: WidgetView<Writer>>(
    state: &Writer,
    layout: &Layout,
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    let galley = state.visible_workspace() != Workspace::Map;
    paper(
        state,
        (&layout.grid, layout.script_pane),
        flex_col((
            zstack((content, galley.then(|| folio::head(state)).flatten()))
                .alignment(UnitPoint::TOP)
                .flex(1.),
            galley.then(|| folio::footer(state)).flatten(),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(0.px()),
    )
}

/// The paper around `content`, centred in a pane `pane` wide: a sheet `grid` wide with a rule edge
/// and a hard stock shadow, set on the desk, or full-bleed paper filling the pane. It reaches the
/// pane's foot.
pub(in crate::view) fn paper<V: WidgetView<Writer>>(
    state: &Writer,
    (grid, pane): (&Grid, f64),
    content: V,
) -> impl WidgetView<Writer> + use<V> {
    let sheet = grid.sheet;
    let paper = sized_box(content)
        // One Dimensions for both axes: a later `height` alone would reset the width to auto.
        .dims(
            Dimensions::AUTO
                .with_width(grid.width.px())
                .with_height(Dim::Stretch),
        )
        .background_color(state.palette.surface)
        .border_color(if sheet {
            state.palette.rule
        } else {
            Color::TRANSPARENT
        })
        .border_width(if sheet { 1.px() } else { 0.px() })
        .box_shadow(BoxShadow::new(
            if sheet {
                state.palette.stock
            } else {
                Color::TRANSPARENT
            },
            (SHADOW, SHADOW),
        ));
    flex_row((paper,))
        .main_axis_alignment(MainAxisAlignment::Center)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        // The workspace's own 16px padding is the desk above the sheet.
        .padding(if sheet {
            Padding {
                bottom: (space::KINDS + SHADOW).px(),
                ..Padding::all(0.px())
            }
        } else {
            Padding::all(0.px())
        })
        .width(pane.px())
}

#[cfg(test)]
mod tests;
