//! Reject queued native mutations that no longer own the single Workbench draft.
use crate::{Phase, Writer, attention::EditingTarget};
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::widgets::{TextArea, TextInput},
};

pub(super) struct Guarded<V> {
    target: EditingTarget,
    accepted: String,
    input: V,
}
pub(super) fn guard<V>(target: EditingTarget, accepted: String, input: V) -> Guarded<V> {
    Guarded {
        target,
        accepted,
        input,
    }
}
impl<V> ViewMarker for Guarded<V> {}
impl<V: WidgetView<Writer, Widget = TextInput>> View<Writer, (), ViewCtx> for Guarded<V> {
    type Element = Pod<TextInput>;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut Writer) -> (Self::Element, Self::ViewState) {
        self.input.build(ctx, state)
    }
    fn rebuild(
        &self,
        prev: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        widget: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) {
        self.input.rebuild(&prev.input, local, ctx, widget, state);
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        widget: Mut<'_, Self::Element>,
    ) {
        self.input.teardown(local, ctx, widget);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut widget: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) -> MessageResult<()> {
        if self.target.belongs_to(state)
            && state.session.model().view() == &self.target.view()
            && state.phase() == Phase::Writing
            && !state.session.read_only()
        {
            self.input.message(local, message, widget, state)
        } else {
            // The host rejects normal input before mutation. This also repairs a
            // stale/forged native action without creating an untracked second draft.
            TextArea::reset_text(&mut TextInput::text_mut(&mut widget), &self.accepted);
            MessageResult::Action(())
        }
    }
}
