use super::*;
use crate::Writer;
use recite_writer_session::map::camera::Camera;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::properties::Dimensions,
};

pub(in crate::map) struct InputView<V, F> {
    kind: Kind,
    vim: bool,
    child: V,
    callback: F,
    disabled: bool,
    focus: Option<FocusRequest>,
    camera: Option<(Camera, bool)>,
}
pub(in crate::map) fn input<V, F>(kind: Kind, vim: bool, child: V, callback: F) -> InputView<V, F> {
    InputView {
        kind,
        vim,
        child,
        callback,
        disabled: false,
        focus: None,
        camera: None,
    }
}
impl<V, F> InputView<V, F> {
    pub fn focus(mut self, focus: Option<FocusRequest>) -> Self {
        self.focus = focus;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Lets a viewport translate its content locally between camera commits.
    pub fn camera(mut self, committed: Camera, panning: bool) -> Self {
        self.camera = Some((committed, panning));
        self
    }
}
impl<V, F> ViewMarker for InputView<V, F> {}
impl<V: WidgetView<Writer>, F: Fn(&mut Writer, Input) + Send + Sync + 'static>
    View<Writer, (), ViewCtx> for InputView<V, F>
{
    type Element = Pod<InputWidget>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut Writer) -> (Self::Element, Self::ViewState) {
        let (child, local) = ctx.with_id(ViewId::new(0), |ctx| self.child.build(ctx, state));
        let pod = ctx.with_action_widget(|_| {
            let mut widget = InputWidget::new(child.new_widget, self.kind.clone(), self.vim);
            widget.focus = self.focus.clone();
            if let Some((camera, _)) = self.camera {
                widget.pan = super::pan::LocalPan::committed(camera);
            }
            let mut pod = Pod::new_with_props(widget, Dimensions::MAX);
            pod.new_widget.options.disabled = self.disabled;
            pod
        });
        (pod, local)
    }
    fn rebuild(
        &self,
        prev: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) {
        if self.disabled != prev.disabled {
            element.ctx.set_disabled(self.disabled);
        }
        let restore_focus = self.kind == Kind::Viewport(Detail::Overview)
            && prev.kind != self.kind
            && element.ctx.has_focus_target();
        InputWidget::configure(&mut element, &self.kind, self.vim);
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = InputWidget::child_mut(&mut element);
            self.child
                .rebuild(&prev.child, local, ctx, child.downcast(), state);
        });
        if restore_focus {
            InputWidget::restore_overview_focus(&mut element);
        }
        InputWidget::configure_focus(&mut element, self.focus.as_ref());
        if let Some((camera, panning)) = self.camera {
            InputWidget::configure_camera(&mut element, camera, panning);
        }
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(ViewId::new(0), |ctx| {
            let mut child = InputWidget::child_mut(&mut element);
            self.child.teardown(local, ctx, child.downcast());
        });
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) -> MessageResult<()> {
        if let Some(id) = message.take_first() {
            if id != ViewId::new(0) {
                return MessageResult::Stale;
            }
            let mut child = InputWidget::child_mut(&mut element);
            return self.child.message(local, message, child.downcast(), state);
        }
        let Some(action) = message.take_message::<NativeAction>() else {
            return MessageResult::Stale;
        };
        let NativeAction::Input(input) = *action else {
            return MessageResult::Stale;
        };
        (self.callback)(state, input);
        MessageResult::Action(())
    }
}
