//! Native emphasis preserves text order, shaping and accessibility for comparisons and search.
use super::ShapedText;
use super::styled_text::Family;
use recite_writer_session::comparison::changed_range;
use std::num::NonZeroUsize;
use std::ops::Range;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, StyleProperty},
        parley::FontWeight,
        properties::LineBreaking,
    },
};

use recite_writer_app::presentation::emphasis as search;
#[derive(PartialEq)]
enum Emphasis {
    ChangedFrom(ArcStr),
    Search(ArcStr),
}

pub(crate) fn search_label(size: f32, text: String, query: String) -> EmphasizedText {
    search_text(size, text, query, Family::Ui)
}
fn search_text(size: f32, text: String, query: String, family: Family) -> EmphasizedText {
    EmphasizedText {
        text: text.into(),
        emphasis: Emphasis::Search(query.into()),
        size,
        family,
        single: false,
    }
}
pub(crate) struct EmphasizedText {
    text: ArcStr,
    emphasis: Emphasis,
    size: f32,
    family: Family,
    /// One line ending in an ellipsis; assistive technology still reads the whole text.
    single: bool,
}
impl EmphasizedText {
    pub fn single_line(mut self) -> Self {
        self.single = true;
        self
    }
}
pub(crate) fn manuscript_comparison(size: f32, text: String, other: String) -> EmphasizedText {
    EmphasizedText {
        text: text.into(),
        emphasis: Emphasis::ChangedFrom(other.into()),
        size,
        family: Family::Manuscript,
        single: false,
    }
}
pub(crate) fn source_comparison(size: f32, text: String, other: String) -> EmphasizedText {
    EmphasizedText {
        text: text.into(),
        emphasis: Emphasis::ChangedFrom(other.into()),
        size,
        family: Family::Source,
        single: false,
    }
}
impl EmphasizedText {
    fn styles(&self) -> Vec<(Range<usize>, StyleProperty)> {
        let ranges = match &self.emphasis {
            Emphasis::ChangedFrom(other) => vec![changed_range(&self.text, other)],
            Emphasis::Search(query) => search::ranges(&self.text, query),
        };
        ranges
            .into_iter()
            .filter(|range| !range.is_empty())
            .flat_map(|range| {
                [
                    (range.clone(), StyleProperty::FontWeight(FontWeight::BOLD)),
                    (range, StyleProperty::Underline(true)),
                ]
            })
            .collect()
    }
}
impl ViewMarker for EmphasizedText {}
impl<State: 'static> View<State, (), ViewCtx> for EmphasizedText {
    type Element = Pod<ShapedText>;
    type ViewState = ();
    fn build(&self, _: &mut ViewCtx, _: &mut State) -> (Self::Element, ()) {
        let label = ShapedText::new(self.text.clone())
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::FontFamily(self.family.name().into()))
            .with_ranged_styles(self.styles())
            .with_max_lines(self.single.then_some(NonZeroUsize::MIN));
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
        if self.text != previous.text || self.emphasis != previous.emphasis {
            ShapedText::set_text(&mut element, self.text.clone());
            ShapedText::set_ranged_styles(&mut element, self.styles());
        }
        if self.family != previous.family {
            ShapedText::insert_style(
                &mut element,
                StyleProperty::FontFamily(self.family.name().into()),
            );
        }
        if self.size != previous.size {
            ShapedText::insert_style(&mut element, StyleProperty::FontSize(self.size));
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
