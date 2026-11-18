//! Let a control's hit target reach into the spacing around its row: the child keeps its full
//! height and baselines while the row counts it `reach` shorter, so a tall target beside short
//! text no longer spreads the rows apart.
#[cfg(test)]
mod tests;
mod widget;
use widget::Overhang;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
};

pub(crate) struct Overhung<V>(V, f64, f64);
/// `child`, `reach` taller than its row counts it: half the reach above and half below.
pub(crate) fn overhang<V>(child: V, reach: f64) -> Overhung<V> {
    Overhung(child, reach, 0.5)
}
/// `child`, `reach` taller than its row counts it, all of the reach below, such as a caption
/// that hangs from a label into the spacing under its row.
pub(crate) fn hang_below<V>(child: V, reach: f64) -> Overhung<V> {
    Overhung(child, reach, 0.)
}
impl<V> ViewMarker for Overhung<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Overhung<V> {
    type Element = Pod<Overhang>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.0.build(ctx, state));
        (
            Pod::new_with_props(
                Overhang::new(child.new_widget, self.1, self.2),
                Dimensions::AUTO,
            ),
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
                Overhang::child_mut(&mut pod).downcast(),
                state,
            );
        });
        if (self.1, self.2) != (previous.1, previous.2) {
            Overhang::set_reach(&mut pod, self.1, self.2);
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
                .teardown(local, ctx, Overhang::child_mut(&mut pod).downcast())
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
            Overhang::child_mut(&mut pod).downcast(),
            state,
        )
    }
}
