//! Transform an owned container, independently of its child's transforms or state.
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{kurbo::Affine, properties::Dimensions, widgets::Passthrough},
};

pub(super) struct Placed<V> {
    child: V,
    affine: Affine,
}
pub(super) fn placed<V>(affine: Affine, child: V) -> Placed<V> {
    Placed { child, affine }
}
impl<V> ViewMarker for Placed<V> {}
impl<State: 'static, V: WidgetView<State>> View<State, (), ViewCtx> for Placed<V> {
    type Element = Pod<Passthrough>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let (child, child_state) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        let mut pod = Pod::new_with_props(Passthrough::new(child.new_widget), Dimensions::MAX);
        pod.new_widget.options.transform = self.affine;
        (pod, child_state)
    }
    fn rebuild(
        &self,
        prev: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = Passthrough::child_mut(&mut element);
            self.child
                .rebuild(&prev.child, local, ctx, child.downcast(), state);
        });
        if self.affine != prev.affine {
            let id = element.id();
            element.ctx.set_transform(self.affine);
            ctx.mark_transform_changed(id);
        }
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = Passthrough::child_mut(&mut element);
            self.child.teardown(local, ctx, child.downcast());
        });
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut State,
    ) -> MessageResult<()> {
        if message.take_first().is_none_or(|id| id != ViewId::new(0)) {
            return MessageResult::Stale;
        }
        let mut child = Passthrough::child_mut(&mut element);
        self.child.message(local, message, child.downcast(), state)
    }
}

#[cfg(test)]
mod tests;
