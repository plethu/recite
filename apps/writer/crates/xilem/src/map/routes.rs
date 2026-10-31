//! Route drawing keeps viewport bounds but paints into the surrounding overscan.
//!
//! Masonry's Canvas clips to its own size. Local panning translates retained content by up to
//! `OVERSCAN` before committing the camera, so routes and the dot grid must already be drawn
//! past each viewport edge. This canvas leaves clipping to the Map viewport; its accessible node
//! still reports the visible viewport.
mod grid;
pub(super) use grid::Grid;

use crate::Writer;
use recite_writer_session::map::topology::OVERSCAN;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        accesskit::{Node, Role},
        core::{
            AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
            RegisterCtx, Widget,
        },
        imaging::{Painter, record::Scene},
        kurbo::{Axis, Size},
        layout::{LenReq, Length},
    },
};

const DESCRIPTION: &str = "Dialogue routes; use Connections below for their accessible list.";

pub(super) struct RoutesWidget {
    scene: Scene,
    width: Length,
    grid: Grid,
    tile: Option<grid::Tile>,
}
impl Widget for RoutesWidget {
    type Action = NoAction;
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn measure(
        &mut self,
        _: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _: Option<Length>,
    ) -> Length {
        match (axis, len_req) {
            (Axis::Horizontal, _) => self.width,
            (Axis::Vertical, LenReq::FitContent(space)) => space,
            (Axis::Vertical, _) => Length::ZERO,
        }
    }
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _: Size) {}
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let area = ctx
            .border_box()
            .inflate(f64::from(OVERSCAN), f64::from(OVERSCAN));
        grid::paint(self.grid, ctx.scale_factor(), area, &mut self.tile, painter);
        painter.replay(&self.scene);
    }
    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_description(DESCRIPTION);
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

/// Re-records the routes from accepted Writer state on every rebuild, like Xilem's canvas.
pub(super) struct Routes {
    width: Length,
    grid: Grid,
    draw: fn(&Writer, &mut Scene),
}
pub(super) fn routes(width: Length, grid: Grid, draw: fn(&Writer, &mut Scene)) -> Routes {
    Routes { width, grid, draw }
}
impl ViewMarker for Routes {}
impl View<Writer, (), ViewCtx> for Routes {
    type Element = Pod<RoutesWidget>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, state: &mut Writer) -> (Self::Element, Self::ViewState) {
        let mut scene = Scene::new();
        (self.draw)(state, &mut scene);
        let widget = RoutesWidget {
            scene,
            width: self.width,
            grid: self.grid,
            tile: None,
        };
        (ctx.create_pod(widget), ())
    }
    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) {
        if self.width != prev.width {
            element.widget.width = self.width;
            element.ctx.request_layout();
        }
        element.widget.grid = self.grid;
        element.widget.scene.clear();
        (self.draw)(state, &mut element.widget.scene);
        element.ctx.request_paint_only();
    }
    fn teardown(&self, (): &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    fn message(
        &self,
        (): &mut Self::ViewState,
        _: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut Writer,
    ) -> MessageResult<()> {
        MessageResult::Stale
    }
}
