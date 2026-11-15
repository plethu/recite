//! A running head: a line laid over the top of a scrolled sheet that shows only once the page has
//! scrolled past its own title. The native host reads the scroll and shows or hides it, so the
//! line never takes space from the page or moves the text beneath it.
#[cfg(test)]
mod tests;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{
        accesskit::{Node, Role},
        core::{
            AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
            PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut,
            WidgetPod,
        },
        imaging::Painter,
        kurbo::{Axis, Point, Size},
        layout::{AsUnit, LenDef, LenReq, Length},
    },
};

pub(crate) struct RunningHead {
    shown: bool,
    child: WidgetPod<dyn Widget>,
}
impl RunningHead {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> Self {
        Self {
            shown: false,
            child: child.erased().to_pod(),
        }
    }
    pub fn is_shown(&self) -> bool {
        self.shown
    }
    /// Show or hide the line; hidden, it is stashed, so it neither paints nor reads aloud.
    pub fn set_shown(this: &mut WidgetMut<'_, Self>, shown: bool) {
        if this.widget.shown != shown {
            this.widget.shown = shown;
            this.ctx.set_stashed(&mut this.widget.child, !shown);
            this.ctx.request_layout();
        }
    }
    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }
}
impl Widget for RunningHead {
    type Action = NoAction;
    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            ctx.set_stashed(&mut self.child, !self.shown);
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
        match axis {
            // Hidden, the line takes no height; shown, only its own, however tall the sheet.
            Axis::Vertical if !self.shown => 0.px(),
            Axis::Vertical => {
                let context = ctx.context_size();
                ctx.compute_length(&mut self.child, LenDef::MaxContent, context, axis, cross)
            }
            Axis::Horizontal => ctx.redirect_measurement(&mut self.child, axis, cross),
        }
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, size: Size) {
        if self.shown {
            ctx.run_layout(&mut self.child, size);
            ctx.place_child(&mut self.child, Point::ORIGIN);
        }
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

pub(crate) struct Running<V> {
    child: V,
}
/// `child` as a running head, hidden until the native host shows it.
pub(crate) fn running_head<V>(child: V) -> Running<V> {
    Running { child }
}
impl<V> ViewMarker for Running<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Running<V> {
    type Element = Pod<RunningHead>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        (ctx.create_pod(RunningHead::new(child.new_widget)), local)
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
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                RunningHead::child_mut(&mut element).downcast(),
                state,
            );
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child
                .teardown(local, ctx, RunningHead::child_mut(&mut element).downcast());
        });
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_first() != Some(ViewId::new(0)) {
            return MessageResult::Stale;
        }
        self.child.message(
            local,
            message,
            RunningHead::child_mut(&mut element).downcast(),
            state,
        )
    }
}
