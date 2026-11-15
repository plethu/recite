//! The error wedge: a 24px target that names a problem and carries its diagnostic as the
//! accessible description, so a screen reader hears what is wrong before acting on it.
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        accesskit::{Action, Node, Role},
        core::{
            AccessCtx, AccessEvent, ArcStr, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx,
            PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx,
            Widget, WidgetMut,
            keyboard::{Key, NamedKey},
        },
        imaging::Painter,
        kurbo::{Axis, BezPath, Size, Stroke},
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
}
impl Wedge {
    pub fn new(name: ArcStr, description: ArcStr, (ink, ring): (Color, Color)) -> Self {
        Self {
            name,
            description,
            ink,
            ring,
        }
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
        SIDE.px()
    }
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _: Size) {}
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let edge = ctx.border_box();
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
        Role::Button
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(&*self.name);
        node.set_description(&*self.description);
        node.add_action(Action::Click);
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
    fn accepts_focus(&self) -> bool {
        true
    }
}

pub(crate) struct WedgeView<F> {
    name: ArcStr,
    description: ArcStr,
    inks: (Color, Color),
    press: F,
}
/// A wedge named `name` describing `description` in the error ink, with a focus ring in `ring`.
pub(crate) fn wedge<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    name: impl Into<ArcStr>,
    description: impl Into<ArcStr>,
    (ink, ring): (Color, Color),
    press: F,
) -> WedgeView<F> {
    WedgeView {
        name: name.into(),
        description: description.into(),
        inks: (ink, ring),
        press,
    }
}
impl<F> ViewMarker for WedgeView<F> {}
impl<S: 'static, F: Fn(&mut S) + Send + Sync + 'static> View<S, (), ViewCtx> for WedgeView<F> {
    type Element = Pod<Wedge>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let widget = Wedge::new(self.name.clone(), self.description.clone(), self.inks);
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }
    fn rebuild(
        &self,
        old: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if old.name != self.name || old.description != self.description || old.inks != self.inks {
            Wedge::set(
                &mut element,
                self.name.clone(),
                self.description.clone(),
                self.inks,
            );
        }
    }
    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        _: &mut (),
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_message::<WedgePress>().is_some() {
            (self.press)(state);
            MessageResult::Action(())
        } else {
            MessageResult::Stale
        }
    }
}
