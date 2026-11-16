use xilem::masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
        PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Axis, Point, Size},
    layout::{LenReq, Length},
};

/// Lays its child out `reach` taller than the height it reports, the extra split evenly above
/// and below, so the child's target overlaps its neighbours' spacing without adding to it.
pub(crate) struct Overhang {
    child: WidgetPod<dyn Widget>,
    reach: f64,
}
impl Overhang {
    pub fn new(child: NewWidget<impl Widget + ?Sized>, reach: f64) -> Self {
        Self {
            child: child.erased().to_pod(),
            reach: reach.max(0.),
        }
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    pub fn set_reach(this: &mut WidgetMut<'_, Self>, reach: f64) {
        this.widget.reach = reach.max(0.);
        this.ctx.request_layout();
    }
}
impl Widget for Overhang {
    type Action = NoAction;
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }
    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        axis: Axis,
        _: LenReq,
        cross: Option<Length>,
    ) -> Length {
        let length = ctx.redirect_measurement(&mut self.child, axis, cross);
        match axis {
            Axis::Horizontal => length,
            Axis::Vertical => Length::px((length.get() - self.reach).max(0.)),
        }
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(
            &mut self.child,
            Size::new(size.width, size.height + self.reach),
        );
        ctx.place_child(&mut self.child, Point::new(0., -self.reach / 2.));
        ctx.derive_baselines(&self.child);
    }
    fn paint(&mut self, _: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, _: &mut Painter<'_>) {}
    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut Node) {}
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
