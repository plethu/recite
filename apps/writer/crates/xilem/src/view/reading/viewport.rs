//! Page changes reset the native scroll position without replacing manuscript widgets.
use crate::Writer;
use recite_core::DocumentKey;
use xilem::{
    ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{core::Widget, kurbo::Point, widgets::Portal},
};
#[cfg(test)]
mod tests;

#[derive(PartialEq)]
struct Page {
    document: DocumentKey,
    beat: Option<String>,
    index: usize,
}
pub(in crate::view) struct FollowPage<V> {
    page: Page,
    child: V,
}
pub(in crate::view) fn follow_page<V>(writer: &Writer, child: V) -> FollowPage<V> {
    FollowPage {
        page: Page {
            document: writer.session.model().document().key().clone(),
            beat: writer.reading.selected.clone(),
            index: writer.reading.page,
        },
        child,
    }
}
impl<V> ViewMarker for FollowPage<V> {}
impl<W: Widget, V: WidgetView<Writer, Widget = Portal<W>>> View<Writer, (), ViewCtx>
    for FollowPage<V>
{
    type Element = V::Element;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut Writer) -> (Self::Element, Self::ViewState) {
        self.child.build(ctx, state)
    }
    fn rebuild(
        &self,
        previous: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut widget: Mut<'_, Self::Element>,
        state: &mut Writer,
    ) {
        self.child
            .rebuild(&previous.child, local, ctx, widget.reborrow_mut(), state);
        if self.page != previous.page {
            Portal::set_viewport_pos(&mut widget, Point::ORIGIN);
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
        state: &mut Writer,
    ) -> MessageResult<()> {
        self.child.message(local, message, widget, state)
    }
}
