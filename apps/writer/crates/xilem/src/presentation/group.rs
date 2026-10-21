//! Name a composed native pane without owning its scrolling, focus or product state.
#[cfg(test)]
mod tests;
mod widget;
use widget::Group;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{core::ArcStr, properties::Dimensions},
};

pub(crate) struct Named<V> {
    name: ArcStr,
    child: V,
}
pub(crate) fn named<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        child,
    }
}
impl<V> ViewMarker for Named<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Named<V> {
    type Element = Pod<Group>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        (
            Pod::new_with_props(
                Group::new(self.name.clone(), child.new_widget),
                Dimensions::MAX,
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
        if self.name != previous.name {
            Group::set_name(&mut pod, self.name.clone());
        }
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child.rebuild(
                &previous.child,
                local,
                ctx,
                Group::child_mut(&mut pod).downcast(),
                state,
            )
        });
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            self.child
                .teardown(local, ctx, Group::child_mut(&mut pod).downcast())
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
        self.child
            .message(local, message, Group::child_mut(&mut pod).downcast(), state)
    }
}
