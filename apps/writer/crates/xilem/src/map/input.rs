//! Local native input delivery; the widget owns capture, never authoring or file I/O.
use super::Detail;
mod events;
mod focus;
mod press;
mod view;
pub(super) use focus::{FocusRequest, FocusToken};
pub(super) use view::input;
use xilem::masonry::widgets;
use xilem::masonry::{
    accesskit::{Action, Node, Role},
    core::{
        self, AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
        PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update, UpdateCtx, Widget,
        WidgetMut, WidgetPod,
    },
    imaging::Painter,
    kurbo::{Axis, Point, Size, Stroke, Vec2},
    layout::{LenReq, Length},
    properties::BorderColor,
};
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Kind {
    Viewport(Detail),
    Grip(String),
    Card,
}
#[derive(Debug)]
pub(super) enum Input {
    Begin(Point),
    Move(Point),
    Finish,
    OverviewClick(Point),
    Cancel,
    Pan(Vec2),
    Zoom { factor: f32, anchor: Point },
    Direction(Vec2),
    Open,
    Search,
    Focus,
    Hover(bool),
    Focused(FocusToken),
}
#[derive(Debug)]
pub(super) enum NativeAction {
    Input(Input),
    FocusTarget {
        target: core::WidgetId,
        token: FocusToken,
    },
}
impl From<Input> for NativeAction {
    fn from(input: Input) -> Self {
        Self::Input(input)
    }
}
#[derive(Clone, Copy)]
struct PointerPress {
    button: Option<xilem::view::PointerButton>,
    motion: Motion,
}
#[derive(Clone, Copy)]
enum Motion {
    ClickCandidate(Point),
    Dragging,
}
pub(super) struct InputWidget {
    child: WidgetPod<dyn Widget>,
    kind: Kind,
    vim: bool,
    press: Option<PointerPress>,
    focus: Option<FocusRequest>,
    clicks: u8,
}
impl InputWidget {
    fn new(child: NewWidget<impl Widget + ?Sized>, kind: Kind, vim: bool) -> Self {
        Self {
            child: child.erased().to_pod(),
            kind,
            vim,
            press: None,
            focus: None,
            clicks: 0,
        }
    }
    fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    fn configure(this: &mut WidgetMut<'_, Self>, kind: &Kind, vim: bool) {
        if this.widget.kind != *kind {
            this.widget.kind = kind.clone();
            this.ctx.request_accessibility_update();
        }
        this.widget.vim = vim;
    }
}
impl Widget for InputWidget {
    type Action = NativeAction;
    fn on_action(
        &mut self,
        ctx: &mut core::ActionCtx<'_>,
        _: &mut PropertiesMut<'_>,
        action: &core::ErasedAction,
        source: core::WidgetId,
    ) {
        if self.kind == Kind::Card
            && source == self.child.id()
            && let Some(press) = action.downcast_ref::<widgets::ButtonPress>()
        {
            let clicks = std::mem::take(&mut self.clicks);
            if clicks == 2 && press.button == Some(xilem::view::PointerButton::Primary) {
                ctx.submit_action::<NativeAction>(Input::Open);
                ctx.set_handled();
                return;
            }
        }
        self.overview_focus_action(ctx, action, source);
        self.focus_action(ctx, action);
    }
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }
    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &core::PointerEvent,
    ) {
        self.pointer(ctx, event);
    }
    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        self.keyboard(ctx, event);
    }
    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if ctx.target() == ctx.widget_id() && event.action == Action::Click {
            ctx.request_focus();
            ctx.set_handled();
        }
    }
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if *event == Update::WidgetAdded {
            self.request_card_focus(ctx);
        }
        if matches!(event, Update::FocusChanged(_)) {
            ctx.request_paint_only();
        }
        if self.kind == Kind::Card && *event == Update::ChildFocusChanged(true) {
            ctx.submit_action::<NativeAction>(Input::Focus);
        }
        if self.kind == Kind::Card
            && let Update::ChildHoveredChanged(hovered) = event
        {
            ctx.submit_action::<NativeAction>(Input::Hover(*hovered));
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
        if matches!(self.kind, Kind::Viewport(_)) {
            ctx.set_clip_path(size.to_rect());
        }
    }
    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        if ctx.is_focus_target() {
            let color = props.get::<BorderColor>(ctx.property_cache()).color;
            painter
                .stroke(ctx.border_box().inset(-1.), &Stroke::new(2.), color)
                .draw();
        }
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
    fn accepts_focus(&self) -> bool {
        self.kind != Kind::Card
    }
    fn accessibility_role(&self) -> Role {
        if matches!(self.kind, Kind::Grip(_)) {
            Role::Button
        } else {
            Role::GenericContainer
        }
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        match &self.kind {
            Kind::Viewport(detail) => {
                node.set_label("Scene map");
                if *detail == Detail::Overview {
                    node.set_description("Structural overview. Click or use arrows to select a beat at reading size. Enter opens Script; Go to and the outline reach every beat.");
                    return;
                }
                node.set_description("Arrow keys select a beat. Enter opens its script. Drag empty space to pan; Control plus scroll zooms.");
            }
            Kind::Grip(title) => {
                node.set_label(format!("Move {title}"));
                node.set_description(
                    "Drag to move this beat, or use arrow keys. Escape cancels a drag.",
                );
                node.add_action(Action::Click);
            }
            Kind::Card => {}
        }
    }
}

#[cfg(test)]
mod tests;
