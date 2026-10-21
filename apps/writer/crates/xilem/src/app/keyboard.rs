//! A tagged immutable keyboard context routes native host messages through Xilem.
use super::*;
use crate::attention::{DocumentMenu, EditingTarget, PaneMotion};
use crate::localisation::TranslationTarget;
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
    pub screen: Option<crate::workspaces::Screen>,
    pub picker_open: bool,
    pub document_menu: DocumentMenu,
    pub attention: crate::attention::Attention,
    pub editing: Option<EditingTarget>,
    pub field_restoration: Option<EditingTarget>,
    pub keymap: recite_config::Keymap,
    pub proxy: MessageProxy<Invocation>,
}
pub(crate) struct Invocation {
    pub action: Action,
    pub generation: CommandGeneration,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Command(WriterCommand),
    SaveTranslation(TranslationTarget),
    SubmitPicker,
    PaneHint(bool),
    CyclePane { forward: bool },
    FieldFocus(Option<EditingTarget>),
    DismissDocumentMenu,
}
impl std::fmt::Debug for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInvocation")
            .field("action", &self.action)
            .finish_non_exhaustive()
    }
}
#[derive(PartialEq)]
struct Scope {
    writer: Option<DocumentScope>,
    welcome: bool,
    settings: bool,
    picker: Option<(picker::PickerInstance, picker::Mode, picker::PickerRevision)>,
    keymap: recite_config::Keymap,
}
struct DocumentScope {
    window: WindowId,
    view: recite_writer_model::View,
    workspace: crate::map::Workspace,
    stamp: recite_writer_model::AnalysisStamp,
    attention: crate::attention::Attention,
    editing: Option<EditingTarget>,
    field_restoration: Option<EditingTarget>,
    writing: bool,
    screen: crate::workspaces::Screen,
    document_menu: DocumentMenu,
}
impl PartialEq for DocumentScope {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
            && self.view == other.view
            && self.workspace == other.workspace
            && self.stamp == other.stamp
            && self.attention == other.attention
            && self.editing == other.editing
            && self.field_restoration == other.field_restoration
            && self.writing == other.writing
            && self.screen == other.screen
            && self.document_menu == other.document_menu
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
                    workspace: writer.map.workspace,
                    stamp: model.document().analysis_stamp(),
                    attention: writer.attention.clone(),
                    editing: EditingTarget::capture(writer),
                    field_restoration: writer.field_restoration(),
                    writing: writer.phase == Phase::Writing,
                    screen: writer.workspaces.screen,
                    document_menu: writer.document_menu,
                }
            }),
            welcome: app.show_welcome,
            settings: app.settings_open,
            keymap: app.preferences.config.ui.keymap,
            picker: app
                .picker
                .as_ref()
                .map(|picker| (picker.instance.clone(), picker.mode(), picker.revision)),
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
            writer_active: self.scope.writer.as_ref().is_some_and(|writer| {
                writer.writing && writer.screen == crate::workspaces::Screen::Write
            }) && !self.scope.welcome
                && !self.scope.settings,
            picker_open: self.scope.picker.is_some(),
            document_menu: self
                .scope
                .writer
                .as_ref()
                .map_or(DocumentMenu::Closed, |writer| writer.document_menu),
            screen: self
                .scope
                .writer
                .as_ref()
                .filter(|writer| writer.writing && !self.scope.welcome && !self.scope.settings)
                .map(|writer| writer.screen),
            attention: self
                .scope
                .writer
                .as_ref()
                .map(|writer| writer.attention.clone())
                .unwrap_or_default(),
            editing: self
                .scope
                .writer
                .as_ref()
                .filter(|writer| {
                    writer.writing
                        && writer.screen == crate::workspaces::Screen::Write
                        && !self.scope.welcome
                        && !self.scope.settings
                        && self.scope.picker.is_none()
                })
                .and_then(|writer| writer.editing.clone()),
            keymap: self.scope.keymap,
            field_restoration: self
                .scope
                .writer
                .as_ref()
                .and_then(|writer| writer.field_restoration.clone()),
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
        if invocation.generation != state.generation
            || self.scope != Scope::capture(app)
            || self.shortcuts != app.preferences.config.writer.shortcuts
        {
            return MessageResult::Stale;
        }
        match invocation.action {
            Action::FieldFocus(target) => {
                if let Some(writer) = &mut app.writer {
                    writer.native_field_focus(target);
                }
            }
            Action::Command(command) => app.run_command(command),
            Action::SaveTranslation(target) => {
                if let Some(writer) = &mut app.writer {
                    writer.save_focused_translation(&target);
                }
            }
            Action::SubmitPicker => app.submit_picker(),
            Action::DismissDocumentMenu => {
                if let Some(writer) = &mut app.writer {
                    writer.document_menu = DocumentMenu::Dismissed;
                }
            }
            Action::PaneHint(pending) => app.pane_hint = pending,
            Action::CyclePane { forward } => {
                if let Some(writer) = &mut app.writer {
                    writer.focus_pane(if forward {
                        PaneMotion::Next
                    } else {
                        PaneMotion::Previous
                    });
                }
            }
        }
        MessageResult::Action(())
    }
}

#[cfg(test)]
mod tests;
