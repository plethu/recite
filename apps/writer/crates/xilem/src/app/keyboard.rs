//! A tagged immutable keyboard context routes native host messages through Xilem.
use super::*;
use recite_config::{WriterCommand, WriterShortcuts};
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
/// Freshness of commands captured from one mounted keyboard context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CommandGeneration(u64);
impl CommandGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
#[derive(Clone, Default)]
pub(crate) struct KeyboardContext(pub Option<Arc<Context>>);
impl Property for KeyboardContext {
    fn static_default() -> &'static Self {
        static EMPTY: KeyboardContext = KeyboardContext(None);
        &EMPTY
    }
}
pub(crate) struct Context {
    pub shortcuts: WriterShortcuts,
    pub generation: CommandGeneration,
    pub writer_active: bool,
    pub picker_open: bool,
    pub proxy: MessageProxy<Invocation>,
}
pub(crate) struct Invocation {
    pub command: WriterCommand,
    pub generation: CommandGeneration,
}
impl std::fmt::Debug for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInvocation")
            .field("command", &self.command)
            .finish_non_exhaustive()
    }
}
#[derive(PartialEq)]
struct Scope {
    writer: Option<DocumentScope>,
    welcome: bool,
    settings: bool,
    picker: Option<(picker::Mode, picker::PickerRevision)>,
}
struct DocumentScope {
    window: WindowId,
    view: recite_writer_model::View,
    stamp: recite_writer_model::AnalysisStamp,
}
impl PartialEq for DocumentScope {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window && self.view == other.view && self.stamp == other.stamp
    }
}
impl Scope {
    fn capture(app: &App) -> Self {
        Self {
            writer: app.writer.as_ref().map(|writer| {
                let model = writer.session.model();
                DocumentScope {
                    window: writer.window,
                    view: model.view().clone(),
                    stamp: model.document().analysis_stamp(),
                }
            }),
            welcome: app.show_welcome,
            settings: app.settings_open,
            picker: app
                .picker
                .as_ref()
                .map(|picker| (picker.mode, picker.revision)),
        }
    }
}
pub(super) struct Host<V> {
    child: V,
    scope: Scope,
    shortcuts: WriterShortcuts,
}
pub(super) struct Hosted<T> {
    child: T,
    generation: CommandGeneration,
    proxy: MessageProxy<Invocation>,
}
pub(super) fn host<V: WidgetView<App>>(app: &App, child: V) -> Host<V> {
    Host {
        child,
        scope: Scope::capture(app),
        shortcuts: app.preferences.config.writer.shortcuts.clone(),
    }
}
impl<V> Host<V> {
    fn context(&self, state: &Hosted<impl Sized>) -> KeyboardContext {
        KeyboardContext(Some(Arc::new(Context {
            shortcuts: self.shortcuts.clone(),
            generation: state.generation,
            writer_active: self.scope.writer.is_some()
                && !self.scope.welcome
                && !self.scope.settings,
            picker_open: self.scope.picker.is_some(),
            proxy: state.proxy.clone(),
        })))
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
            generation: CommandGeneration::default(),
            proxy,
        };
        let mut pod = Pod::new_with_props(Passthrough::new(child.new_widget), Dimensions::MAX);
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
        if self.scope != prev.scope || self.shortcuts != prev.shortcuts {
            state.generation.advance();
        }
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
        if invocation.generation != state.generation
            || self.scope != Scope::capture(app)
            || self.shortcuts != app.preferences.config.writer.shortcuts
        {
            return MessageResult::Stale;
        }
        app.run_command(invocation.command);
        MessageResult::Action(())
    }
}

#[cfg(test)]
mod tests;
