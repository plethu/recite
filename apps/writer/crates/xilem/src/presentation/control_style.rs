//! Native style classes retain focus and hover without replacing control behavior.
use xilem::{
    ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlStyle {
    Action,
    Navigation,
    Segment,
    Manuscript,
}
impl ControlStyle {
    pub fn class(self) -> &'static str {
        match self {
            Self::Action => "writer-action",
            Self::Navigation => "writer-navigation",
            Self::Segment => "writer-segment",
            Self::Manuscript => "writer-manuscript",
        }
    }
}
pub(crate) struct Styled<V> {
    style: ControlStyle,
    child: V,
}
pub(crate) fn styled<V>(style: ControlStyle, child: V) -> Styled<V> {
    Styled { style, child }
}
impl<V> ViewMarker for Styled<V> {}
impl<S: 'static, V: WidgetView<S>> View<S, (), ViewCtx> for Styled<V> {
    type Element = V::Element;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut S) -> (Self::Element, Self::ViewState) {
        let (mut pod, local) = self.child.build(ctx, state);
        pod.new_widget = pod.new_widget.with_class(self.style.class());
        (pod, local)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut widget: Mut<'_, Self::Element>,
        state: &mut S,
    ) {
        self.child
            .rebuild(&previous.child, local, ctx, widget.reborrow_mut(), state);
        if self.style != previous.style {
            widget.ctx.remove_class(previous.style.class());
            widget.ctx.add_class(self.style.class());
        }
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        widget: Mut<'_, Self::Element>,
    ) {
        self.child.teardown(local, ctx, widget);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        widget: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        self.child.message(local, message, widget, state)
    }
}
