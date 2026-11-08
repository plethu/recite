//! Explicit jumps reset scrolling; overlapping reading shifts retain their native anchor.
use crate::Writer;
use recite_writer_app::reading::Anchor;
use xilem::{
    ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{Property, UsesProperty, Widget},
        kurbo::Point,
        widgets::Portal,
    },
};
#[cfg(test)]
mod tests;

/// The native scroll container's copy of the reading anchor it was built for.
#[derive(Clone, Default, PartialEq)]
pub(crate) struct ReadingViewport(Anchor);
impl ReadingViewport {
    pub(crate) fn capture(writer: &Writer) -> Self {
        Self(Anchor::capture(writer))
    }
    pub(crate) fn shifted_from(&self, previous: &Self) -> bool {
        self.0.shifted_from(&previous.0)
    }
    pub(crate) fn is_manuscript(&self) -> bool {
        self.0.is_manuscript()
    }
}
impl Property for ReadingViewport {
    fn static_default() -> &'static Self {
        static EMPTY: ReadingViewport = ReadingViewport(Anchor::NONE);
        &EMPTY
    }
}
impl<W: Widget> UsesProperty<ReadingViewport> for Portal<W> {}
pub(in crate::view) struct FollowReading<V> {
    page: ReadingViewport,
    child: V,
}
pub(in crate::view) fn follow_reading<V>(writer: &Writer, child: V) -> FollowReading<V> {
    FollowReading {
        page: ReadingViewport::capture(writer),
        child,
    }
}
impl<V> ViewMarker for FollowReading<V> {}
impl<W: Widget, V: WidgetView<Writer, Widget = Portal<W>>> View<Writer, (), ViewCtx>
    for FollowReading<V>
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
        let shifted = self.page.shifted_from(&previous.page);
        if self.page != previous.page && !shifted {
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
