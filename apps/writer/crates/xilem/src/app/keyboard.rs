//! A tagged immutable keyboard context routes native host messages to the shell through Xilem.
use super::App;
use crate::workspaces::rename::inline::Target;
use recite_config::WriterShortcuts;
pub(crate) use recite_writer_app::shell::keyboard::*;
use std::sync::Arc;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{
        MessageCtx, MessageProxy, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker,
    },
    masonry::{
        core::{Property, UsesProperty, WidgetTag},
        properties::Dimensions,
        widgets::{Button, Passthrough, SizedBox, TextInput},
    },
};

pub(crate) const TAG: WidgetTag<Passthrough> = WidgetTag::named("recite-commands");
use recite_writer_app::shell::keyboard::Context as CommandContext;
pub(crate) type Context = CommandContext<MessageProxy<Invocation>>;
#[derive(Clone, Default)]
pub(crate) struct KeyboardContext(pub Option<Arc<Context>>);

/// Marks the control a modal surface focuses when it opens, such as "Keep editing" on the close
/// decision or the Settings sheet's close action.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct DefaultFocus(pub bool);
impl Property for DefaultFocus {
    fn static_default() -> &'static Self {
        static OTHER: DefaultFocus = DefaultFocus(false);
        &OTHER
    }
}
impl UsesProperty<DefaultFocus> for Button {}
/// A field can be the default too, such as a beat heading renamed in place.
impl UsesProperty<DefaultFocus> for TextInput {}

/// Marks a control whose picker opens as a combobox slip hanging from it, such as a reply's
/// destination or a line's speaker.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct PickerAnchor(pub bool);
impl Property for PickerAnchor {
    fn static_default() -> &'static Self {
        static OTHER: PickerAnchor = PickerAnchor(false);
        &OTHER
    }
}
impl UsesProperty<PickerAnchor> for Button {}

/// Marks a sidebar beat row or map card: F2 on it renames its beat in place there.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct RenamesBeat(pub Option<Target>);
impl Property for RenamesBeat {
    fn static_default() -> &'static Self {
        static NONE: RenamesBeat = RenamesBeat(None);
        &NONE
    }
}
impl UsesProperty<RenamesBeat> for Button {}

/// Marks the control focus returns to when a modal surface closes and the control focused before
/// it was rebuilt meanwhile, such as the sidebar row whose name became the rename field.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ReturnFocus(pub bool);
impl Property for ReturnFocus {
    fn static_default() -> &'static Self {
        static OTHER: ReturnFocus = ReturnFocus(false);
        &OTHER
    }
}
impl UsesProperty<ReturnFocus> for Button {}

/// Marks a modal surface: while one is mounted, Tab and Shift+Tab cycle within it.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct ModalSurface(pub bool);
impl Property for ModalSurface {
    fn static_default() -> &'static Self {
        static OTHER: ModalSurface = ModalSurface(false);
        &OTHER
    }
}
impl UsesProperty<ModalSurface> for SizedBox {}
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
