//! The error wedge: a 24px target that names a problem and carries its diagnostic as the
//! accessible description, so a screen reader hears what is wrong before acting on it. Where a
//! problem is repeated in a row, card or message, the same widget paints the error sign instead.
mod view;
pub(crate) use view::{wedge, wedge_glyph};
use xilem::{
    masonry::{
        accesskit::{Action, Node, Role},
        core::{
            AccessCtx, AccessEvent, ArcStr, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
            PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx,
            Widget, WidgetMut,
            keyboard::{Key, NamedKey},
        },
        imaging::Painter,
        kurbo::{Axis, BezPath, Cap, Circle, Line, Size, Stroke},
        layout::{AsUnit, LenReq, Length},
    },
    peniko::Color,
};

/// A wedge's side: the least target size.
pub(crate) const SIDE: f64 = 24.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WedgePress;

pub(crate) struct Wedge {
    name: ArcStr,
    description: ArcStr,
    ink: Color,
    ring: Color,
    /// The side of a decorative wedge, which repeats a problem inside a control that already says
    /// so; `None` for the focusable 24px control.
    glyph: Option<f64>,
}
impl Wedge {
    pub fn new(name: ArcStr, description: ArcStr, (ink, ring): (Color, Color)) -> Self {
        Self {
            name,
            description,
            ink,
            ring,
            glyph: None,
        }
    }
    /// A decorative wedge `side` square in `ink`: neither focusable nor read aloud.
    pub fn glyph(side: f64, ink: Color) -> Self {
        Self {
            glyph: Some(side),
            ..Self::new(ArcStr::from(""), ArcStr::from(""), (ink, ink))
        }
    }
    /// Whether this is the focusable control rather than a decorative glyph.
    pub fn is_control(&self) -> bool {
        self.glyph.is_none()
    }
    /// Resize and re-ink a decorative wedge.
    pub fn set_glyph(this: &mut WidgetMut<'_, Self>, side: f64, ink: Color) {
        this.widget.glyph = Some(side);
        (this.widget.ink, this.widget.ring) = (ink, ink);
        this.ctx.request_layout();
    }
    pub fn set(
        this: &mut WidgetMut<'_, Self>,
        name: ArcStr,
        description: ArcStr,
        inks: (Color, Color),
    ) {
        this.widget.name = name;
        this.widget.description = description;
        (this.widget.ink, this.widget.ring) = inks;
        this.ctx.request_accessibility_update();
        this.ctx.request_paint_only();
    }
}
impl Widget for Wedge {
    type Action = WedgePress;
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if self.glyph.is_some() {
            return;
        }
        match event {
            PointerEvent::Down(..) => {
                ctx.request_focus();
                ctx.capture_pointer();
            }
            PointerEvent::Up(..) if ctx.is_active() && ctx.is_hovered() => {
                ctx.submit_action::<WedgePress>(WedgePress);
            }
            _ => {}
        }
    }
    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        if let TextEvent::Keyboard(event) = event
            && event.state.is_up()
            && (matches!(&event.key, Key::Character(c) if c == " ")
                || event.key == Key::Named(NamedKey::Enter))
        {
            ctx.submit_action::<WedgePress>(WedgePress);
        }
    }
    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if event.action == Action::Click {
            ctx.submit_action::<WedgePress>(WedgePress);
        }
    }
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::FocusChanged(_) | Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn measure(
        &mut self,
        _: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        _: Axis,
        _: LenReq,
        _: Option<Length>,
    ) -> Length {
        self.glyph.unwrap_or(SIDE).px()
    }
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _: Size) {}
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let edge = ctx.border_box();
        if let Some(side) = self.glyph {
            // The repeated mark is the standard error sign, a circled exclamation mark, so it
            // never reads as a disclosure or play arrow.
            let weight = (side / 10.).max(1.5);
            let (centre, radius) = (edge.center(), side / 2. - weight / 2.);
            painter
                .stroke(Circle::new(centre, radius), &Stroke::new(weight), self.ink)
                .draw();
            let bar = Line::new(
                (centre.x, centre.y - 0.5 * radius),
                (centre.x, centre.y + 0.1 * radius),
            );
            painter
                .stroke(bar, &Stroke::new(weight).with_caps(Cap::Round), self.ink)
                .draw();
            painter
                .fill(
                    Circle::new((centre.x, centre.y + 0.45 * radius), 0.6 * weight),
                    self.ink,
                )
                .draw();
            return;
        }
        // A wedge pointing into the text: a filled triangle in the error ink.
        let mut wedge = BezPath::new();
        wedge.move_to((edge.x0 + 7., edge.y0 + 6.));
        wedge.line_to((edge.x1 - 6., edge.y0 + edge.height() / 2.));
        wedge.line_to((edge.x0 + 7., edge.y1 - 6.));
        wedge.close_path();
        painter.fill(wedge, self.ink).draw();
        if ctx.is_focus_target() {
            painter
                .stroke(edge.inset(-1.), &Stroke::new(2.), self.ring)
                .draw();
        } else if ctx.is_hovered() {
            painter
                .stroke(edge.inset(-1.), &Stroke::new(1.), self.ink)
                .draw();
        }
    }
    fn accessibility_role(&self) -> Role {
        if self.glyph.is_some() {
            Role::GenericContainer
        } else {
            Role::Button
        }
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        if self.glyph.is_some() {
            node.set_hidden();
            return;
        }
        node.set_label(&*self.name);
        node.set_description(&*self.description);
        node.add_action(Action::Click);
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
    fn accepts_focus(&self) -> bool {
        self.glyph.is_none()
    }
}
