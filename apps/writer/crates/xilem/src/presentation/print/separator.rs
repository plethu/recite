//! Visual separators between status phrases stay out of the accessible text.
use xilem::{
    ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::widgets,
    peniko::Color,
    style::Style,
};

/// A middle dot drawn like surrounding metadata, hidden from assistive technology so
/// adjacent labels read as separate phrases rather than as punctuation.
pub(crate) fn separator<State: 'static>(
    scale: f32,
    color: Color,
) -> Separator<impl WidgetView<State, Widget = widgets::Label>> {
    Separator(super::meta(scale, "·").color(color))
}

pub(crate) struct Separator<V>(V);
impl<V> ViewMarker for Separator<V> {}
impl<State: 'static, V: WidgetView<State, Widget = widgets::Label>> View<State, (), ViewCtx>
    for Separator<V>
{
    type Element = V::Element;
    type ViewState = V::ViewState;
    fn build(&self, ctx: &mut ViewCtx, state: &mut State) -> (Self::Element, Self::ViewState) {
        let (mut pod, local) = self.0.build(ctx, state);
        let label = std::mem::replace(&mut *pod.new_widget.widget, widgets::Label::new(""));
        *pod.new_widget.widget = label.accessibility_hidden(true);
        (pod, local)
    }
    fn rebuild(
        &self,
        prev: &Self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        state: &mut State,
    ) {
        self.0.rebuild(&prev.0, local, ctx, element, state);
    }
    fn teardown(
        &self,
        local: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        self.0.teardown(local, ctx, element);
    }
    fn message(
        &self,
        local: &mut Self::ViewState,
        message: &mut MessageCtx,
        element: Mut<'_, Self::Element>,
        state: &mut State,
    ) -> MessageResult<()> {
        self.0.message(local, message, element, state)
    }
}
