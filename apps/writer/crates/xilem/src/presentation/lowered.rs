//! Move a layer down from where its parent lays it out. Padding would keep the band above it in
//! the layer's own bounds, where it takes the pointer from whatever lies beneath; a layer moved
//! down leaves that band to the window under it.
//!
//! Xilem's own `transformed` cannot hold the app's state, which is not `Send`.
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::kurbo::Affine,
};

pub(crate) struct Lowered<V> {
    child: V,
    by: f64,
}
/// `child`, laid out where its parent puts it and then moved `by` pixels down.
pub(crate) fn lowered<V>(by: f64, child: V) -> Lowered<V> {
    Lowered { child, by }
}
impl<V> ViewMarker for Lowered<V> {}
impl<S: 'static, A: 'static, V: WidgetView<S, A>> View<S, A, ViewCtx> for Lowered<V> {
    type Element = Pod<V::Widget>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (mut pod, local) = self.child.build(ctx, state);
        pod.new_widget.options.transform = Affine::translate((0., self.by));
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
        self.child
            .rebuild(&previous.child, local, ctx, element.reborrow_mut(), state);
        if self.by != previous.by {
            element.ctx.set_transform(Affine::translate((0., self.by)));
        }
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(local, ctx, element);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<A> {
        self.child.message(local, message, element, state)
    }
}
