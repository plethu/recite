use xilem::masonry::{
    accesskit::{Live, Node, Role},
    core::{
        AccessCtx, ArcStr, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
        PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Axis, Point, Size},
    layout::{LenReq, Length},
};
pub(crate) struct Group {
    name: ArcStr,
    role: Role,
    child: WidgetPod<dyn Widget>,
}
impl Group {
    pub fn new(name: ArcStr, role: Role, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            name,
            role,
            child: child.erased().to_pod(),
        }
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    pub fn set_name(this: &mut WidgetMut<'_, Self>, name: ArcStr) {
        this.widget.name = name;
        this.ctx.request_accessibility_update();
    }
    pub fn set_role(this: &mut WidgetMut<'_, Self>, role: Role) {
        this.widget.role = role;
        this.ctx.request_accessibility_update();
    }
}
impl Widget for Group {
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
        ctx.redirect_measurement(&mut self.child, axis, cross)
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        ctx.derive_baselines(&self.child);
    }
    fn paint(&mut self, _: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, _: &mut Painter<'_>) {}
    fn accessibility_role(&self) -> Role {
        self.role
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.name.as_ref());
        node.set_live(if self.role == Role::Status {
            Live::Polite
        } else {
            Live::Off
        });
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
