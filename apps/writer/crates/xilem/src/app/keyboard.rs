//! A tagged immutable keyboard context routes native host messages to the shell through Xilem.
use super::App;
use recite_config::WriterShortcuts;
pub(crate) use recite_writer_app::shell::keyboard::*;
use std::sync::Arc;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{
        MessageCtx, MessageProxy, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker,
    },
    masonry::{
        core::{Property, WidgetTag},
        properties::Dimensions,
        widgets::Passthrough,
    },
};

pub(crate) const TAG: WidgetTag<Passthrough> = WidgetTag::named("recite-commands");
use recite_writer_app::shell::keyboard::Context as CommandContext;
pub(crate) type Context = CommandContext<MessageProxy<Invocation>>;
#[derive(Clone, Default)]
pub(crate) struct KeyboardContext(pub Option<Arc<Context>>);
impl Property for KeyboardContext {
    fn static_default() -> &'static Self {
        static EMPTY: KeyboardContext = KeyboardContext(None);
        &EMPTY
    }
}
pub(super) struct Host<V> {
    child: V,
    scope: Scope,
    shortcuts: WriterShortcuts,
}
pub(super) struct Hosted<T> {
    child: T,
    generations: Generations,
    proxy: MessageProxy<Invocation>,
}
pub(super) fn host<V: WidgetView<App>>(app: &App, child: V) -> Host<V> {
    Host {
        child,
        scope: Scope::capture(app),
        shortcuts: app.preferences.config.writer.shortcuts.clone(),
    }
}
impl<V> ViewMarker for Host<V> {}
impl<V: WidgetView<App>> View<App, (), ViewCtx> for Host<V> {
    type Element = Pod<Passthrough>;
    type ViewState = Hosted<V::ViewState>;
    fn build(&self, ctx: &mut ViewCtx, app: &mut App) -> (Self::Element, Self::ViewState) {
        let proxy = MessageProxy::new(ctx.proxy(), ctx.view_path().into());
        let (child, child_state) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, app));
        let state = Hosted {
            child: child_state,
            generations: Generations::default(),
            proxy,
        };
        let mut pod = Pod::new_with_props(Passthrough::new(child.new_widget), Dimensions::MAX);
        ctx.record_action_source(pod.new_widget.id());
        pod.new_widget = pod
            .new_widget
            .with_tag(TAG)
            .with_props(self.context(&state));
        (pod, state)
    }
    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app: &mut App,
    ) {
        state.generations.update(
            (&prev.scope, &prev.shortcuts),
            (&self.scope, &self.shortcuts),
        );
        if self.scope != prev.scope || self.shortcuts != prev.shortcuts {
            element.insert_prop(self.context(state));
        }
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = Passthrough::child_mut(&mut element);
            self.child
                .rebuild(&prev.child, &mut state.child, ctx, child.downcast(), app);
        });
    }
    fn teardown(
        &self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = Passthrough::child_mut(&mut element);
            self.child.teardown(&mut state.child, ctx, child.downcast());
        });
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app: &mut App,
    ) -> MessageResult<()> {
        if let Some(id) = message.take_first() {
            if id.routing_id() != 0 {
                return MessageResult::Stale;
            }
            let mut child = Passthrough::child_mut(&mut element);
            return self
                .child
                .message(&mut state.child, message, child.downcast(), app);
        }
        let Some(invocation) = message.take_message::<Invocation>() else {
            return MessageResult::Stale;
        };
        if app.invoke(&self.scope, &self.shortcuts, state.generations, *invocation) {
            MessageResult::Action(())
        } else {
            MessageResult::Stale
        }
    }
}

impl<V> Host<V> {
    fn context(&self, state: &Hosted<impl Sized>) -> KeyboardContext {
        KeyboardContext(Some(Arc::new(self.scope.context(
            &self.shortcuts,
            state.generations,
            state.proxy.clone(),
        ))))
    }
}

#[cfg(test)]
mod tests;
