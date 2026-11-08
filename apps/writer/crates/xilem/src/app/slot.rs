//! Optional native child lifecycle; stale messages never need a fabricated Writer.
use super::*;
use crate::Writer;
use xilem::{
    Pod, ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewId, ViewMarker, ViewPathTracker},
    masonry::{
        core::NewWidget,
        properties::Dimensions,
        widgets::{Passthrough, SizedBox},
    },
};

pub(super) struct WriterSlot<V> {
    key: Option<recite_writer_app::WindowKey>,
    child: Option<V>,
}
pub(super) struct SlotState<T> {
    child: Option<T>,
    child_id: ViewId,
}
pub(super) fn writer_slot(app: &mut App) -> WriterSlot<impl WidgetView<Writer> + use<>> {
    WriterSlot {
        key: app.writer.as_ref().map(|writer| writer.window),
        child: app.writer.as_ref().map(crate::view::writer),
    }
}
impl<V> ViewMarker for WriterSlot<V> {}
impl<V: WidgetView<Writer>> View<App, (), ViewCtx> for WriterSlot<V> {
    type Element = Pod<Passthrough>;
    type ViewState = SlotState<V::ViewState>;
    fn build(&self, ctx: &mut ViewCtx, app: &mut App) -> (Self::Element, Self::ViewState) {
        if let (Some(child), Some(writer)) = (&self.child, &mut app.writer) {
            let (child, state) = ctx.with_id(ViewId::new(0), |ctx| child.build(ctx, writer));
            (
                Pod::new_with_props(Passthrough::new(child.new_widget), Dimensions::MAX),
                SlotState {
                    child: Some(state),
                    child_id: ViewId::new(0),
                },
            )
        } else {
            (
                Pod::new_with_props(
                    Passthrough::new(NewWidget::new(SizedBox::empty())),
                    Dimensions::MAX,
                ),
                SlotState {
                    child: None,
                    child_id: ViewId::new(0),
                },
            )
        }
    }
    fn rebuild(
        &self,
        prev: &Self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app: &mut App,
    ) {
        if self.key == prev.key
            && let (Some(child), Some(previous), Some(child_state), Some(writer)) =
                (&self.child, &prev.child, &mut state.child, &mut app.writer)
        {
            ctx.with_id(state.child_id, |ctx| {
                let mut pod = Passthrough::child_mut(&mut element);
                child.rebuild(previous, child_state, ctx, pod.downcast(), writer);
            });
            return;
        }
        if self.key == prev.key && self.child.is_none() && prev.child.is_none() {
            return;
        }
        prev.teardown(state, ctx, element.reborrow_mut());
        state.child_id = ViewId::new(state.child_id.routing_id().wrapping_add(1));
        if let (Some(child), Some(writer)) = (&self.child, &mut app.writer) {
            let (child, next) = ctx.with_id(state.child_id, |ctx| child.build(ctx, writer));
            Passthrough::set_child(&mut element, child.new_widget);
            state.child = Some(next);
        } else {
            Passthrough::set_child(&mut element, NewWidget::new(SizedBox::empty()));
            state.child = None;
        }
    }
    fn teardown(
        &self,
        state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        if let (Some(child), Some(child_state)) = (&self.child, &mut state.child) {
            ctx.with_id(state.child_id, |ctx| {
                let mut pod = Passthrough::child_mut(&mut element);
                child.teardown(child_state, ctx, pod.downcast());
            });
        }
        state.child = None;
    }
    fn message(
        &self,
        state: &mut Self::ViewState,
        message: &mut MessageCtx,
        mut element: Mut<'_, Self::Element>,
        app: &mut App,
    ) -> MessageResult<()> {
        if message.take_first().is_none_or(|id| id != state.child_id) {
            return MessageResult::Stale;
        }
        let (Some(child), Some(child_state), Some(writer)) =
            (&self.child, &mut state.child, &mut app.writer)
        else {
            return MessageResult::Stale;
        };
        if self.key != Some(writer.window) {
            return MessageResult::Stale;
        }
        let result = {
            let mut pod = Passthrough::child_mut(&mut element);
            child.message(child_state, message, pod.downcast(), writer)
        };
        if let Some(text) = writer.pending_clipboard.take() {
            element.ctx.set_clipboard(text);
            writer.clipboard_written();
        }
        writer.after_native_message();
        app.apply_writer_requests();
        result
    }
}

#[cfg(test)]
mod tests;
