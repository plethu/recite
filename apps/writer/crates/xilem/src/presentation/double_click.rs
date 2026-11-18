//! A double-click on a button asks for something else than its press, such as a sidebar beat
//! row renaming its beat in place while a single press opens it. The wrapper watches the clicks
//! that reach the button and turns the second press of a primary double-click into its own
//! action; the keyboard reaches the same command another way, such as F2.
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{
        accesskit::{Node, Role},
        core::{
            AccessCtx, ActionCtx, ChildrenIds, ErasedAction, EventCtx, LayoutCtx, MeasureCtx,
            NewWidget, PaintCtx, PointerButton, PointerEvent, PropertiesMut, PropertiesRef,
            RegisterCtx, Widget, WidgetId, WidgetMut, WidgetPod,
        },
        imaging::Painter,
        kurbo::{Axis, Point, Size},
        layout::{LenReq, Length},
        properties::Dimensions,
        widgets::ButtonPress,
    },
};

/// The action a double-click submits in place of the second press.
#[derive(Debug)]
pub(crate) struct DoublePress;

pub(crate) struct DoubleClickWidget {
    child: WidgetPod<dyn Widget>,
    /// The click count of the last primary release inside.
    clicks: u8,
}
impl DoubleClickWidget {
    fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}
impl Widget for DoubleClickWidget {
    type Action = DoublePress;
    fn on_pointer_event(
        &mut self,
        _: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        match event {
            PointerEvent::Up(event) if event.button == Some(PointerButton::Primary) => {
                self.clicks = event.state.count;
            }
            PointerEvent::Up(_) | PointerEvent::Down(_) | PointerEvent::Cancel(_) => {
                self.clicks = 0;
            }
            _ => {}
        }
    }
    fn on_action(
        &mut self,
        ctx: &mut ActionCtx<'_>,
        _: &mut PropertiesMut<'_>,
        action: &ErasedAction,
        _: WidgetId,
    ) {
        let Some(press) = action.downcast_ref::<ButtonPress>() else {
            return;
        };
        if std::mem::take(&mut self.clicks) == 2 && press.button == Some(PointerButton::Primary) {
            ctx.submit_action::<DoublePress>(DoublePress);
            ctx.set_handled();
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
        _: LenReq,
        cross: Option<Length>,
    ) -> Length {
        ctx.redirect_measurement(&mut self.child, axis, cross)
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
    }
    fn paint(&mut self, _: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, _: &mut Painter<'_>) {}
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut Node) {}
}

pub(crate) struct DoubleClick<V, F> {
    child: V,
    double: F,
}
/// `child`, a button or a view around one, calling `double` instead of its press on the second
/// click of a primary double-click.
pub(crate) fn double_click<V, F>(child: V, double: F) -> DoubleClick<V, F> {
    DoubleClick { child, double }
}
impl<V, F> ViewMarker for DoubleClick<V, F> {}
impl<S: 'static, V: WidgetView<S>, F: Fn(&mut S) + Send + Sync + 'static> View<S, (), ViewCtx>
    for DoubleClick<V, F>
{
    type Element = Pod<DoubleClickWidget>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        let pod = ctx.with_action_widget(|_| {
            Pod::new_with_props(
                DoubleClickWidget {
                    child: NewWidget::erased(child.new_widget).to_pod(),
                    clicks: 0,
                },
                Dimensions::AUTO,
            )
        });
        (pod, local)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = DoubleClickWidget::child_mut(&mut element);
            self.child
                .rebuild(&previous.child, local, ctx, child.downcast(), state);
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = DoubleClickWidget::child_mut(&mut element);
            self.child.teardown(local, ctx, child.downcast());
        });
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if let Some(id) = message.take_first() {
            if id != ViewId::new(0) {
                return MessageResult::Stale;
            }
            let mut child = DoubleClickWidget::child_mut(&mut element);
            return self.child.message(local, message, child.downcast(), state);
        }
        if message.take_message::<DoublePress>().is_none() {
            return MessageResult::Stale;
        }
        (self.double)(state);
        MessageResult::Action(())
    }
}
