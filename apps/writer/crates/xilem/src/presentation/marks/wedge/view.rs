//! The wedge's views: the focusable control that names a problem, and the decorative glyph that
//! repeats it inside a row or card whose name already says so.
use super::{Wedge, WedgePress};
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::core::ArcStr,
    peniko::Color,
};

pub(crate) struct WedgeView<F> {
    name: ArcStr,
    description: ArcStr,
    inks: (Color, Color),
    press: F,
}
/// A wedge named `name` describing `description` in the error ink, with a focus ring in `ring`.
pub(crate) fn wedge<S: 'static, F: Fn(&mut S) + Send + Sync + 'static>(
    name: impl Into<ArcStr>,
    description: impl Into<ArcStr>,
    (ink, ring): (Color, Color),
    press: F,
) -> WedgeView<F> {
    WedgeView {
        name: name.into(),
        description: description.into(),
        inks: (ink, ring),
        press,
    }
}
impl<F> ViewMarker for WedgeView<F> {}
impl<S: 'static, F: Fn(&mut S) + Send + Sync + 'static> View<S, (), ViewCtx> for WedgeView<F> {
    type Element = Pod<Wedge>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let widget = Wedge::new(self.name.clone(), self.description.clone(), self.inks);
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }
    fn rebuild(
        &self,
        old: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if old.name != self.name || old.description != self.description || old.inks != self.inks {
            Wedge::set(
                &mut element,
                self.name.clone(),
                self.description.clone(),
                self.inks,
            );
        }
    }
    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_action_source(element);
    }
    fn message(
        &self,
        _: &mut (),
        message: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        state: &mut S,
    ) -> MessageResult<()> {
        if message.take_message::<WedgePress>().is_some() {
            (self.press)(state);
            MessageResult::Action(())
        } else {
            MessageResult::Stale
        }
    }
}

/// A decorative wedge `side` square in `ink`, for a row or card whose name already says there is
/// a problem.
pub(crate) fn wedge_glyph(side: f64, ink: Color) -> GlyphView {
    GlyphView { side, ink }
}
pub(crate) struct GlyphView {
    side: f64,
    ink: Color,
}
impl ViewMarker for GlyphView {}
impl<S: 'static> View<S, (), ViewCtx> for GlyphView {
    type Element = Pod<Wedge>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        (ctx.create_pod(Wedge::glyph(self.side, self.ink)), ())
    }
    fn rebuild(
        &self,
        old: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if old.side != self.side || old.ink != self.ink {
            Wedge::set_glyph(&mut element, self.side, self.ink);
        }
    }
    fn teardown(&self, _: &mut (), _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    fn message(
        &self,
        _: &mut (),
        _: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut S,
    ) -> MessageResult<()> {
        MessageResult::Stale
    }
}
