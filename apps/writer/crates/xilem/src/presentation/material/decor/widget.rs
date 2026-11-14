use super::Decoration;
use xilem::masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
        PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Axis, Point, Size, Stroke},
    layout::{LenReq, Length},
};

/// The ring's width and its distance inside the child's edge, in logical pixels.
pub(super) const RING: f64 = 2.;

pub(crate) struct Decor {
    decoration: Decoration,
    child: WidgetPod<dyn Widget>,
}
impl Decor {
    pub fn new(decoration: Decoration, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            decoration,
            child: child.erased().to_pod(),
        }
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    pub fn set_decoration(this: &mut WidgetMut<'_, Self>, decoration: Decoration) {
        this.widget.decoration = decoration;
        this.ctx.request_paint_only();
        this.ctx.request_post_paint();
    }
}
impl Widget for Decor {
    type Action = NoAction;
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::ChildFocusChanged(_))
            && matches!(self.decoration, Decoration::Ring(_))
        {
            ctx.request_post_paint();
        }
    }
    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        axis: Axis,
        _: LenReq,
        cross: Option<Length>,
    ) -> Length {
        ctx.redirect_measurement(&mut self.child, axis, cross)
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        ctx.derive_baselines(&self.child);
    }
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if let Decoration::Grain(Some(grain)) = &self.decoration {
            // The tile repeats from the desk's origin, so it never moves under the pointer.
            painter.fill(ctx.content_box(), grain).draw();
        }
    }
    fn post_paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if let Decoration::Ring(Some(color)) = self.decoration
            && ctx.has_focus_target()
        {
            // Fill, ring, fill: the ring sits a ring's width inside the edge and is stroked on
            // its centre line, so it never touches the surface around the control.
            let ring = ctx.border_box().inset(-1.5 * RING);
            painter.stroke(ring, &Stroke::new(RING), color).draw();
        }
    }
    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut Node) {}
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
