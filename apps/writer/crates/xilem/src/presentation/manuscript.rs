//! Authored prose uses a stable native text widget with the Writer's reading typeface.
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, NewWidget, StyleProperty},
        widgets::{Prose, TextArea},
    },
};

pub(crate) struct ManuscriptText {
    text: ArcStr,
    size: f32,
}
pub(crate) fn manuscript_text(size: f32, text: impl Into<ArcStr>) -> ManuscriptText {
    ManuscriptText {
        text: text.into(),
        size,
    }
}
impl ViewMarker for ManuscriptText {}
impl<State: 'static> View<State, (), ViewCtx> for ManuscriptText {
    type Element = Pod<Prose>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, ()) {
        let area = TextArea::new_immutable(&self.text)
            .with_word_wrap(true)
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::FontFamily("Literata".into()));
        (
            ctx.create_pod(Prose::from_text_area(NewWidget::new(area)).with_clip(true)),
            (),
        )
    }
    fn rebuild(
        &self,
        previous: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        let mut area = Prose::text_mut(&mut element);
        if self.text != previous.text {
            TextArea::reset_text(&mut area, &self.text);
        }
        if self.size != previous.size {
            TextArea::insert_style(&mut area, StyleProperty::FontSize(self.size));
        }
    }
    fn teardown(&self, _: &mut (), _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    fn message(
        &self,
        _: &mut (),
        _: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<()> {
        MessageResult::Stale
    }
}
