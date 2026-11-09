//! Native map excerpts abbreviate only their visual layout, retaining complete accessible text.
use super::ShapedText;
use std::num::NonZeroUsize;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, StyleProperty},
        parley::LineHeight,
        properties::LineBreaking,
    },
};

#[cfg(test)]
mod tests;

pub(crate) struct Excerpt {
    text: ArcStr,
    size: f32,
    lines: NonZeroUsize,
}
pub(crate) fn excerpt(text: impl Into<ArcStr>, zoom: f32) -> Excerpt {
    Excerpt {
        text: text.into(),
        size: 15_f32.max(14. / zoom),
        lines: NonZeroUsize::MIN.saturating_add(if zoom < 0.8 { 1 } else { 2 }),
    }
}
impl ViewMarker for Excerpt {}
impl<S: 'static> View<S, (), ViewCtx> for Excerpt {
    type Element = Pod<ShapedText>;
    type ViewState = ();
    fn build(&self, _: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let label = ShapedText::new(self.text.clone())
            .with_style(StyleProperty::FontFamily("Literata".into()))
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                1.35,
            )))
            .with_max_lines(Some(self.lines));
        (Pod::new_with_props(label, LineBreaking::WordWrap), ())
    }
    fn rebuild(
        &self,
        previous: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut label: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if self.text != previous.text {
            ShapedText::set_text(&mut label, self.text.clone());
        }
        if self.size != previous.size {
            ShapedText::insert_style(&mut label, StyleProperty::FontSize(self.size));
        }
        if self.lines != previous.lines {
            ShapedText::set_max_lines(&mut label, Some(self.lines));
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
