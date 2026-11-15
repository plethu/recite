//! Read-only manuscript text set at the same metrics as the editable fields, so a beat reads the
//! same whether or not its fields are mounted. Assistive technology reads it as a label in reading
//! order.
use super::ShapedText;
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
}
/// `text` in the reading face at `size`, in `color`.
pub(crate) fn sheet_text(text: impl Into<ArcStr>, size: f32, color: Color) -> SheetText {
    SheetText {
        text: text.into(),
        size,
        color,
    }
}
impl ViewMarker for SheetText {}
impl<S: 'static> View<S, (), ViewCtx> for SheetText {
    type Element = Pod<ShapedText>;
    type ViewState = ();
    fn build(&self, _: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        let text = ShapedText::new(self.text.clone())
            .with_style(StyleProperty::FontFamily(super::print::font(
                recite_writer_session::typography::FONT_PROSE,
            )))
            .with_style(StyleProperty::FontSize(self.size));
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
