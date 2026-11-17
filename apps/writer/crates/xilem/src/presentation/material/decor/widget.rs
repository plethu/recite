use super::{Bar, Decoration, Edge};
use xilem::masonry::kurbo::Rect;
use xilem::masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
        PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Affine, Axis, BezPath, Circle, Point, Shape, Size, Stroke},
    layout::{LenReq, Length},
};

/// The ring's width and its distance inside the child's edge, in logical pixels.
pub(super) const RING: f64 = 2.;
/// A halftone bar's dot pitch, in logical pixels.
const PITCH: f64 = 3.;

/// A column of dots filling `bar`, one per pitch, as wide as the bar.
fn halftone(bar: Rect) -> BezPath {
    let mut dots = BezPath::new();
    let radius = (bar.width().min(bar.height()) / 2.).min(PITCH / 2.) * 0.8;
    let mut y = bar.y0 + PITCH / 2.;
    while y < bar.y1 {
        let mut x = bar.x0 + PITCH / 2.;
        while x < bar.x1 + 0.5 {
            dots.extend(Circle::new((x.min(bar.x1 - radius), y), radius).path_elements(0.1));
            x += PITCH;
        }
        y += PITCH;
    }
    dots
}

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
            && matches!(
                self.decoration,
                Decoration::Ring(_) | Decoration::Outline(..)
            )
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
        if let Decoration::Stock(color, offset) = self.decoration {
            // Two plain fills, no clip and no blur: the stock is a printed edge, not a shadow.
            let size = ctx.border_box();
            for edge in [
                Rect::new(size.x1 - offset, size.y0 + offset, size.x1, size.y1),
                Rect::new(
                    size.x0 + offset,
                    size.y1 - offset,
                    size.x1 - offset,
                    size.y1,
                ),
            ] {
                painter.fill(edge, color).draw();
            }
        }
        if let Decoration::Backdrop(dim, screen) = &self.decoration {
            painter.fill(ctx.content_box(), *dim).draw();
            // The screen repeats from the window's origin, so backdrops that meet, such as the
            // tab row's over the close decision's, print one unbroken screen.
            let origin = ctx.to_local(Point::ORIGIN).to_vec2();
            painter
                .fill(ctx.content_box(), screen)
                .brush_transform(Some(Affine::translate(origin)))
                .draw();
        }
    }
    fn post_paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if let Decoration::Bar(Some(Bar {
            color,
            edge,
            inset,
            lift,
            screen,
            thickness,
        })) = self.decoration
        {
            let border = ctx.border_box();
            let bar = match edge {
                Edge::Bottom => Rect::new(
                    border.x0 + inset,
                    border.y1 - lift - thickness,
                    border.x1 - inset,
                    border.y1 - lift,
                ),
                Edge::Leading => Rect::new(
                    border.x0 + lift,
                    border.y0 + inset,
                    border.x0 + lift + thickness,
                    border.y1 - inset,
                ),
            };
            if screen {
                painter.fill(halftone(bar), color).draw();
            } else {
                painter.fill(bar, color).draw();
            }
        }
        if let Decoration::Ring(Some(color)) = self.decoration
            && ctx.has_focus_target()
        {
            // Fill, ring, fill: the ring sits a ring's width inside the edge and is stroked on
            // its centre line, so it never touches the surface around the control.
            let ring = ctx.border_box().inset(-1.5 * RING);
            painter.stroke(ring, &Stroke::new(RING), color).draw();
        }
        if let Decoration::Outline(Some(color), offset) = self.decoration
            && ctx.has_focus_target()
        {
            // Stroked on its centre line, so its inner edge sits `offset` outside the child.
            let outline = ctx
                .border_box()
                .inset(offset + RING / 2.)
                .to_rounded_rect(offset);
            painter.stroke(outline, &Stroke::new(RING), color).draw();
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
