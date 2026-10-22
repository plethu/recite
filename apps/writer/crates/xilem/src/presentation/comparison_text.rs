//! One native text layout retains shaping and accessibility across emphasised ranges.
use super::styled_text::Family;
use recite_writer_session::comparison::changed_range;
use std::ops::Range;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, StyleProperty},
        parley::FontWeight,
        properties::LineBreaking,
        widgets::Label,
    },
};

pub(crate) struct ComparisonText {
    text: ArcStr,
    other: ArcStr,
    size: f32,
    family: Family,
}
pub(crate) fn manuscript_comparison(size: f32, text: String, other: String) -> ComparisonText {
    ComparisonText {
        text: text.into(),
        other: other.into(),
        size,
        family: Family::Manuscript,
    }
}
pub(crate) fn source_comparison(size: f32, text: String, other: String) -> ComparisonText {
    ComparisonText {
        text: text.into(),
        other: other.into(),
        size,
        family: Family::Source,
    }
}
impl ComparisonText {
    fn styles(&self) -> Vec<(Range<usize>, StyleProperty)> {
        let range = changed_range(&self.text, &self.other);
        if range.is_empty() {
            return vec![];
        }
        vec![
            (range.clone(), StyleProperty::FontWeight(FontWeight::BOLD)),
            (range, StyleProperty::Underline(true)),
        ]
    }
}
impl ViewMarker for ComparisonText {}
impl<State: 'static> View<State, (), ViewCtx> for ComparisonText {
    type Element = Pod<Label>;
    type ViewState = ();
    fn build(&self, _: &mut ViewCtx, _: &mut State) -> (Self::Element, ()) {
        let label = Label::new(self.text.clone())
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::FontFamily(self.family.name().into()))
            .with_ranged_styles(self.styles());
        (Pod::new_with_props(label, LineBreaking::WordWrap), ())
    }
    fn rebuild(
        &self,
        previous: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if self.text != previous.text || self.other != previous.other {
            Label::set_text(&mut element, self.text.clone());
            Label::set_ranged_styles(&mut element, self.styles());
        }
        if self.family != previous.family {
            Label::insert_style(
                &mut element,
                StyleProperty::FontFamily(self.family.name().into()),
            );
        }
        if self.size != previous.size {
            Label::insert_style(&mut element, StyleProperty::FontSize(self.size));
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

#[cfg(test)]
mod tests;
