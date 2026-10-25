//! Explicit jumps reset scrolling; overlapping reading shifts retain their native anchor.
use crate::{Writer, reading::ReadingWindow};
use recite_core::DocumentKey;
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

#[derive(Clone, PartialEq)]
struct ReadingLocation {
    document: DocumentKey,
    beat: Option<String>,
    window: ReadingWindow,
}
#[derive(Clone, Default, PartialEq)]
pub(crate) struct ReadingViewport(Option<ReadingLocation>);
impl ReadingViewport {
    pub(crate) fn capture(writer: &Writer) -> Self {
        Self(Some(ReadingLocation {
            document: writer.session.model().document().key().clone(),
            beat: writer.reading.selected.clone(),
            window: writer.reading.window,
        }))
    }
    pub(crate) fn shifted_from(&self, previous: &Self) -> bool {
        match (&self.0, &previous.0) {
            (Some(current), Some(previous)) => {
                current.document == previous.document
                    && current.beat == previous.beat
                    && current.window.is_shift()
                    && current.window.start() != previous.window.start()
            }
            _ => false,
        }
    }
    pub(crate) fn is_manuscript(&self) -> bool {
        self.0.is_some()
    }
}
impl Property for ReadingViewport {
    fn static_default() -> &'static Self {
        static EMPTY: ReadingViewport = ReadingViewport(None);
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
