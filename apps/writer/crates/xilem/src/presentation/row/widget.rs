//! A pressable list row that tells assistive technology whether it is the selected one.
//!
//! It behaves as Masonry's button does: press with the pointer, Space or Enter, or the Click
//! action. Unlike that button it carries the `selected` state, so a list never needs to spell its
//! selection into the row's name.
use xilem::masonry::{
    accesskit::{Action, Node, Role},
    core::{
        AccessCtx, AccessEvent, ArcStr, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget,
        PaintCtx, PointerButtonEvent, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx,
        TextEvent, Update, UpdateCtx, Widget, WidgetMut, WidgetPod,
        keyboard::{Key, NamedKey},
    },
    imaging::Painter,
    kurbo::{Axis, Point, Size},
    layout::{LayoutSize, LenReq, Length, SizeDef},
};

/// A press on a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowPress;

pub(crate) struct RowButton {
    /// Whether the row is its list's selection; `None` for a pressable row outside any list.
    selected: Option<bool>,
    /// A name that replaces the one read from the row's content.
    name: Option<ArcStr>,
    child: WidgetPod<dyn Widget>,
}
impl RowButton {
    pub fn new(selected: bool, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            selected: Some(selected),
            name: None,
            child: child.erased().to_pod(),
        }
    }
    /// A row that is not part of a list's selection, named `name` rather than by its content.
    pub fn named(name: ArcStr, child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            selected: None,
            name: Some(name),
            child: child.erased().to_pod(),
        }
    }
    /// The row named `name` rather than by its content, where its content is abbreviated or
    /// carries a mark the name has to say aloud.
    pub fn with_name(mut self, name: Option<ArcStr>) -> Self {
        self.name = name;
        self
    }
    pub fn set_name(this: &mut WidgetMut<'_, Self>, name: Option<ArcStr>) {
        this.widget.name = name;
        this.ctx.request_accessibility_update();
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: bool) {
        this.widget.selected = Some(selected);
        this.ctx.request_accessibility_update();
        this.ctx.request_paint_only();
    }
}

impl Widget for RowButton {
    type Action = RowPress;

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
                ctx.request_paint_only();
            }
            PointerEvent::Up(PointerButtonEvent { .. }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    ctx.submit_action::<RowPress>(RowPress);
                }
                ctx.request_paint_only();
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
            ctx.submit_action::<RowPress>(RowPress);
        }
    }

    fn on_access_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &AccessEvent,
    ) {
        if event.action == Action::Click {
            ctx.submit_action::<RowPress>(RowPress);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(
            event,
            Update::HoveredChanged(_) | Update::ActiveChanged(_) | Update::FocusChanged(_)
        ) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        _: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        cross_length: Option<Length>,
    ) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross_length);
        ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross_length)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        // Rows read from the start, so the content keeps the leading edge.
        let child = ctx.compute_size(&mut self.child, SizeDef::fit(size), size.into());
        ctx.run_layout(&mut self.child, child);
        ctx.place_child(
            &mut self.child,
            Point::new(0., (size.height - child.height) / 2.),
        );
        ctx.derive_baselines(&self.child);
    }

    fn paint(&mut self, _: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, _: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.add_action(Action::Click);
        if let Some(selected) = self.selected {
            node.set_selected(selected);
        }
        if let Some(name) = &self.name {
            node.set_label(&**name);
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }

    fn propagates_pointer_interaction(&self) -> bool {
        false
    }

    fn accepts_focus(&self) -> bool {
        true
    }
}
