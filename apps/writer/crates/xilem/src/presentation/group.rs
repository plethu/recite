//! Name a composed native pane without owning its scrolling, focus or product state.
#[cfg(test)]
mod tests;
mod widget;
use widget::Group;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{accesskit::Role, core::ArcStr, properties::Dimensions},
};

pub(crate) struct Named<V> {
    name: ArcStr,
    role: Role,
    disabled: bool,
    child: V,
}
pub(crate) fn named<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        role: Role::Group,
        disabled: false,
        child,
    }
}
pub(crate) fn list<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        role: Role::List,
        disabled: false,
        child,
    }
}
pub(crate) fn status<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        role: Role::Status,
        disabled: false,
        child,
    }
}
pub(crate) fn dialog<V>(name: impl Into<ArcStr>, child: V) -> Named<V> {
    Named {
        name: name.into(),
        role: Role::Dialog,
        disabled: false,
        child,
    }
}
impl<V> Named<V> {
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
impl<V> ViewMarker for Named<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Named<V> {
    type Element = Pod<Group>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        let mut pod = Pod::new_with_props(
            Group::new(self.name.clone(), self.role, child.new_widget),
            Dimensions::MAX,
        );
        pod.new_widget.options.disabled = self.disabled;
        (pod, local)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut pod: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        if self.disabled != previous.disabled {
            pod.ctx.set_disabled(self.disabled);
        }
        if self.name != previous.name {
            Group::set_name(&mut pod, self.name.clone());
        }
        if self.role != previous.role {
            Group::set_role(&mut pod, self.role);
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
