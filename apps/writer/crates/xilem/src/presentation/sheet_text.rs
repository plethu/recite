//! Read-only manuscript text set at the same metrics as the editable fields, so a beat reads the
//! same whether or not its fields are mounted. Assistive technology reads it as a label in reading
//! order.
use super::ShapedText;
use std::num::NonZeroUsize;
use xilem::masonry::TextAlign;
use xilem::masonry::parley::{LineHeight, style::FontWeight};
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, PropertySet, StyleProperty},
        properties::{ContentColor, LineBreaking},
    },
    peniko::Color,
};

pub(crate) struct SheetText {
    text: ArcStr,
    size: f32,
    color: Color,
    families: &'static [&'static str],
    weight: FontWeight,
    line_height: LineHeight,
    /// One line ending in an ellipsis instead of wrapping; the full text is still read aloud.
    single: bool,
    align: TextAlign,
}
/// `text` in the reading face at `size`, in `color`, with the manuscript's leading.
pub(crate) fn sheet_text(text: impl Into<ArcStr>, size: f32, color: Color) -> SheetText {
    SheetText {
        text: text.into(),
        size,
        color,
        families: recite_writer_session::typography::FONT_PROSE,
        weight: FontWeight::NORMAL,
        line_height: super::Sheet::LEADING,
        single: false,
        align: TextAlign::Start,
    }
}
/// `text` set in `role` at `scale` on one line, abbreviated with an ellipsis where it would not
/// fit; assistive technology reads the whole text.
pub(crate) fn one_line(
    text: impl Into<ArcStr>,
    role: super::Type,
    scale: f32,
    color: Color,
) -> SheetText {
    SheetText {
        text: text.into(),
        size: role.size(scale),
        color,
        families: role.families(),
        weight: role.weight(),
        line_height: LineHeight::Absolute(role.metrics().1 * scale),
        single: true,
        align: TextAlign::Start,
    }
}
impl SheetText {
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }
    /// Set at `size` with the manuscript's leading, such as a source line quoted at the sheet's
    /// own reading size.
    pub fn manuscript(mut self, size: f32) -> Self {
        self.size = size;
        self.line_height = super::Sheet::LEADING;
        self
    }
    /// Set against the widget's right edge, such as a caption in the margin column under its
    /// right-aligned label.
    pub fn end(mut self) -> Self {
        self.align = TextAlign::End;
        self
    }
    /// Set in `families` instead of the role's own, such as an id in Plex Mono at Body size.
    pub fn families(mut self, families: &'static [&'static str]) -> Self {
        self.families = families;
        self
    }
}
impl ViewMarker for SheetText {}
impl<S: 'static> View<S, (), ViewCtx> for SheetText {
    type Element = Pod<ShapedText>;
    type ViewState = ();
    fn build(&self, _: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let text = ShapedText::new(self.text.clone())
            .with_style(StyleProperty::FontFamily(super::print::font(self.families)))
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::FontWeight(self.weight))
            .with_style(StyleProperty::LineHeight(self.line_height))
            .with_max_lines(self.single.then_some(NonZeroUsize::MIN))
            .with_align(self.align);
        (
            Pod::new_with_props(
                text,
                PropertySet::new()
                    .with(LineBreaking::WordWrap)
                    .with(ContentColor::new(self.color)),
            ),
            (),
        )
    }
    fn rebuild(
        &self,
        previous: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut text: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if self.text != previous.text {
            ShapedText::set_text(&mut text, self.text.clone());
        }
        if self.size != previous.size {
            ShapedText::insert_style(&mut text, StyleProperty::FontSize(self.size));
        }
        if self.color != previous.color {
            text.insert_prop(ContentColor::new(self.color));
        }
        if self.weight != previous.weight {
            ShapedText::insert_style(&mut text, StyleProperty::FontWeight(self.weight));
        }
        if self.align != previous.align {
            ShapedText::set_align(&mut text, self.align);
        }
        if self.line_height != previous.line_height {
            ShapedText::insert_style(&mut text, StyleProperty::LineHeight(self.line_height));
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
