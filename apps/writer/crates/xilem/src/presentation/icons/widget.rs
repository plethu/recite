//! A decorative image supplies the enclosing native button's accessible name.
use super::Face;
use xilem::masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
        RegisterCtx, Widget, WidgetMut,
    },
    imaging::Painter,
    kurbo::{Affine, Axis, Cap, Join, Size, Stroke},
    layout::{AsUnit, LenReq, Length},
};
pub(crate) struct NativeIcon(pub(super) Face);
impl NativeIcon {
    pub(super) fn set(this: &mut WidgetMut<'_, Self>, face: Face) {
        if this.widget.0.scale != face.scale {
            this.ctx.request_layout();
        }
        if this.widget.0.name != face.name {
            this.ctx.request_accessibility_update();
        }
        this.widget.0 = face;
        this.ctx.request_paint_only();
    }
}
impl Widget for NativeIcon {
    type Action = NoAction;
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn measure(
        &mut self,
        _: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        _: Axis,
        _: LenReq,
        _: Option<Length>,
    ) -> Length {
        (20. * f64::from(self.0.scale)).px()
    }
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _: Size) {}
    fn paint(&mut self, _: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let transform = Affine::scale(f64::from(self.0.scale) * 20. / 24.);
        if self.0.icon.filled() {
            painter
                .fill(self.0.icon.path(), self.0.color)
                .transform(transform)
                .draw();
            return;
        }
        painter
            .stroke(
                self.0.icon.path(),
                &Stroke::new(1.6)
                    .with_caps(Cap::Round)
                    .with_join(Join::Round),
                self.0.color,
            )
            .transform(transform)
            .draw();
    }
    fn accessibility_role(&self) -> Role {
        Role::Image
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.0.name);
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
