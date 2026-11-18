//! Scroll a child into view whenever its key changes, such as the current page of a preview
//! below the transcript it has added to.
#[cfg(test)]
mod tests;
mod widget;
use widget::Reveal;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
};

pub(crate) struct Revealed<V>(V, usize);
/// `child`, scrolled into its viewport each time `key` changes.
pub(crate) fn revealed<V>(key: usize, child: V) -> Revealed<V> {
    Revealed(child, key)
}
impl<V> ViewMarker for Revealed<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Revealed<V> {
    type Element = Pod<Reveal>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.0.build(ctx, state));
        (
            Pod::new_with_props(Reveal::new(child.new_widget), Dimensions::AUTO),
            local,
        )
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.0.rebuild(
                &previous.0,
                local,
                ctx,
                Reveal::child_mut(&mut pod).downcast(),
                state,
            );
        });
        if self.1 != previous.1 {
            Reveal::reveal(&mut pod);
        }
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.0
                .teardown(local, ctx, Reveal::child_mut(&mut pod).downcast())
        });
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_first() != Some(ViewId::new(0)) {
            return MessageResult::Stale;
        }
        self.0.message(
            local,
            message,
            Reveal::child_mut(&mut pod).downcast(),
            state,
        )
    }
}
