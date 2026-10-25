//! Cap a composed scrolling surface at its intrinsic size and the offered viewport.
//! The native Portal owns scrolling; this decorator only prevents empty expansion.
mod widget;
use widget::FitContent;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
};

pub(crate) struct Fitted<V>(V);
pub(crate) fn fit_content<V>(child: V) -> Fitted<V> {
    Fitted(child)
}
impl<V> ViewMarker for Fitted<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Fitted<V> {
    type Element = Pod<FitContent>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.0.build(ctx, state));
        (
            Pod::new_with_props(FitContent::new(child.new_widget), Dimensions::AUTO),
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
                FitContent::child_mut(&mut pod).downcast(),
                state,
            );
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.0
                .teardown(local, ctx, FitContent::child_mut(&mut pod).downcast())
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
            FitContent::child_mut(&mut pod).downcast(),
            state,
        )
    }
}
