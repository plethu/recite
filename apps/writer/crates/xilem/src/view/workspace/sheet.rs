//! Script is a sheet resting on the grained desk, or full-bleed paper where none fits. The sheet
//! is the frame: it stays put and the manuscript scrolls within it, so the sheet always reaches
//! the pane's foot and its running head and foot stay in place.
use super::super::manuscript::folio;
use super::*;
use crate::presentation::grid::Grid;
use crate::presentation::{Page, space};
use recite_writer_app::COMPACT_WINDOW;
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
    let (fill, edge, width, shadow) = face(state, sheet, true);
    let paper = sized_box(content)
        // One Dimensions for both axes: a later `height` alone would reset the width to auto.
        .dims(
            Dimensions::AUTO
                .with_width(grid.width.px())
                .with_height(Dim::Stretch),
        )
        .background_color(fill)
        .border_color(edge)
        .border_width(width.px())
        .box_shadow(shadow);
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

/// The paper's face: its fill, its edge's colour and width, and its stock shadow. A sheet has a
/// rule edge and a hard shadow; full-bleed paper has neither, and no paper leaves the bare desk.
fn face(state: &Writer, sheet: bool, paper: bool) -> (Color, Color, f64, BoxShadow) {
    let fill = if paper {
        state.palette.surface
    } else {
        Color::TRANSPARENT
    };
    let sheet = sheet && paper;
    let (edge, shadow) = if sheet {
        (state.palette.rule, state.palette.stock)
    } else {
        (Color::TRANSPARENT, Color::TRANSPARENT)
    };
    (
        fill,
        edge,
        if sheet { 1. } else { 0. },
        BoxShadow::new(shadow, (SHADOW, SHADOW)),
    )
}

/// The grid of a sheet beside nothing: Preview's and the tool and Localise pages', which rest on
/// the desk wherever Script's sheet would.
pub(crate) fn page_grid(state: &Writer) -> Grid {
    Grid::resolve(
        state.content_width(),
        Page::of(state),
        state.viewport.width >= COMPACT_WINDOW,
    )
}

/// What the workspace's own content sets in: the whole content width while writing, where the
/// sheet is the manuscript's own, or a tool or Localise page's paper inside the sheet's edges and
/// pad, or inside the workspace's padding on full-bleed paper.
pub(crate) fn page_width(state: &Writer) -> f64 {
    let grid = page_grid(state);
    if grid.sheet && state.workspaces.screen != crate::workspaces::Screen::Write {
        grid.width - 2. * (grid.pad + 1.)
    } else {
        state.content_width()
    }
}

/// The padding around a tool or Localise page's content on its paper.
pub(in crate::view) fn page_padding(state: &Writer) -> Padding {
    let grid = page_grid(state);
    if grid.sheet {
        Padding::from_vh(space::GROUPS.px(), grid.pad.px())
    } else {
        Padding {
            bottom: space::KINDS.px(),
            ..Padding::all(space::KINDS.px())
        }
    }
}

/// The workspace `pane` on its paper. Tool and Localise pages rest on the same paper as Script:
/// a sheet on the desk where Script's would rest, else full-bleed. Writing brings its own paper,
/// so there the pane lies on the bare desk. One widget tree serves both, so switching between
/// writing and a tool never replaces the mounted manuscript.
pub(in crate::view) fn page<V: WidgetView<Writer>>(
    state: &Writer,
    writing: bool,
    pane: V,
) -> impl WidgetView<Writer> + use<V> {
    let grid = page_grid(state);
    let sheet = grid.sheet && !writing;
    let (fill, edge, width, shadow) = face(state, grid.sheet, !writing);
    flex_row((sized_box(pane)
        .dims(
            Dimensions::AUTO
                .with_width(if sheet {
                    Dim::Fixed(grid.width.px())
                } else {
                    Dim::Auto
                })
                .with_height(Dim::Stretch),
        )
        .background_color(fill)
        .border_color(edge)
        .border_width(width.px())
        .box_shadow(shadow)
        .flex(if sheet { 0. } else { 1. }),))
    .main_axis_alignment(MainAxisAlignment::Center)
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    // The desk shows above the sheet as it does above Script's, and below it beyond its shadow.
    .padding(if sheet {
        Padding {
            top: space::KINDS.px(),
            bottom: (space::KINDS + SHADOW).px(),
            ..Padding::all(0.px())
        }
    } else {
        Padding::all(0.px())
    })
}

#[cfg(test)]
mod tests;
