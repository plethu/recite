use super::{Bar, Decoration, Marks};
use xilem::masonry::kurbo::Rect;
use xilem::masonry::{
    accesskit::{Node, Role},
    core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
        PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Affine, Axis, Point, Size, Stroke},
    layout::{LenDef, LenReq, Length},
};
use xilem::peniko::{Color, ImageBrush};

/// The ring's width and its distance inside the child's edge, in logical pixels.
pub(super) const RING: f64 = 2.;
/// A bar's thickness, in logical pixels.
const BAR: f64 = 2.;

/// The modal backdrop's dim and print screen over the whole child.
fn backdrop(ctx: &PaintCtx<'_>, painter: &mut Painter<'_>, dim: Color, screen: &ImageBrush) {
    painter.fill(ctx.content_box(), dim).draw();
    // The screen repeats from the window's origin, so backdrops that meet, such as the tab row's
    // over the close decision's, print one unbroken screen.
    let origin = ctx.to_local(Point::ORIGIN).to_vec2();
    painter
        .fill(ctx.content_box(), screen)
        .brush_transform(Some(Affine::translate(origin)))
        .draw();
}

pub(crate) struct Decor {
    decoration: Decoration,
    child: WidgetPod<dyn Widget>,
    /// How wide the child's words run, for an underscore under them: its natural width, at most
    /// its box.
    words: f64,
}
impl Decor {
    pub fn new(decoration: Decoration, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            decoration,
            child: child.erased().to_pod(),
            words: 0.,
        }
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    pub fn set_decoration(this: &mut WidgetMut<'_, Self>, decoration: Decoration) {
        this.widget.decoration = decoration;
        // An underscore needs its words' width, which layout measures.
        this.ctx.request_layout();
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
        self.words = if matches!(
            self.decoration,
            Decoration::Marks(Marks {
                underscore: Some(_),
                ..
            })
        ) {
            ctx.compute_length(
                &mut self.child,
                LenDef::MaxContent,
                size.into(),
                Axis::Horizontal,
                Some(Length::px(size.height)),
            )
            .get()
            .min(size.width)
        } else {
            size.width
        };
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
            backdrop(ctx, painter, *dim, screen);
        }
    }
    fn post_paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if let Decoration::Bar(Some(Bar { color, inset, lift })) = self.decoration {
            let border = ctx.border_box();
            let bar = Rect::new(
                border.x0 + inset,
                border.y1 - lift - BAR,
                border.x1 - inset,
                border.y1 - lift,
            );
            painter.fill(bar, color).draw();
        }
        if let Decoration::Marks(marks) = &self.decoration {
            marks.paint(
                painter,
                ctx.border_box(),
                (ctx.first_baseline(), self.words),
            );
        }
        if let Decoration::Ring(Some(color)) = self.decoration
            && ctx.has_focus_target()
        {
            // Fill, ring, fill: the ring sits a ring's width inside the edge and is stroked on
            // its centre line, so it never touches the surface around the control.
            let ring = ctx.border_box().inset(-1.5 * RING);
            painter.stroke(ring, &Stroke::new(RING), color).draw();
        }
        if let Decoration::Cover(Some((dim, screen))) = &self.decoration {
            backdrop(ctx, painter, *dim, screen);
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
