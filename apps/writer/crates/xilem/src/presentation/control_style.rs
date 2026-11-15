//! Native style classes retain focus and hover without replacing control behavior.
use xilem::{
    ViewCtx, WidgetView,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlStyle {
    /// The one solid plum action on a view.
    Primary,
    /// A secondary action on a sheet, panel or dialog: ink at 8% over its host, no border. The
    /// unclassed default looks the same; the class lets an action change emphasis in place.
    Tonal,
    /// A quiet action on the desk: no fill until hovered.
    Action,
    Navigation,
    /// An unchosen segment: quiet on its tonal track.
    Segment,
    /// The chosen segment: the reading surface on the track, with its ink bar painted above.
    Chosen,
    /// A text tab; the chosen one carries an underline painted above it.
    Tab,
    Manuscript,
    /// A field on paper: the default well, named so a field can move between hosts in place.
    Field,
    /// A field on the grained desk, where the paper rule would fall under 3:1.
    DeskField,
    /// A search field, padded for its leading icon.
    SearchField,
    /// A button shaped like a search field on the desk: it opens a search.
    Entry,
    /// A list row at rest.
    Row,
    /// The selected list row: the second-ink plate, with its ink bar painted above.
    RowSelected,
    /// A compact icon face with even padding.
    Icon,
    /// An icon toggle that is on: the pressed sink held, with its ink edge, so the state shows by
    /// shape as well as fill.
    IconOn,
    /// An action on the status bar: quiet on the overprint, with a mint focus border.
    Status,
}
impl ControlStyle {
    pub fn class(self) -> &'static str {
        match self {
            Self::Primary => "writer-primary",
            Self::Tonal => "writer-tonal",
            Self::Action => "writer-action",
            Self::Navigation => "writer-navigation",
            Self::Segment => "writer-segment",
            Self::Chosen => "writer-chosen",
            Self::Tab => "writer-tab",
            Self::Manuscript => "writer-manuscript",
            Self::Field => "writer-field",
            Self::DeskField => "writer-desk-field",
            Self::SearchField => "writer-search-field",
            Self::Icon => "writer-icon",
            Self::IconOn => "writer-icon-on",
            Self::Entry => "writer-entry",
            Self::Row => "writer-row",
            Self::RowSelected => "writer-row-selected",
            Self::Status => "writer-status",
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
