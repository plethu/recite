//! Read-only authored text uses stable native widgets with explicit manuscript or source typography.
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::{
        core::{ArcStr, NewWidget, StyleProperty},
        widgets::{Prose, TextArea},
    },
};

pub(crate) struct StyledText {
    text: ArcStr,
    size: f32,
    family: Family,
}
pub(crate) fn manuscript_text(size: f32, text: impl Into<ArcStr>) -> StyledText {
    StyledText {
        text: text.into(),
        size,
        family: Family::Manuscript,
    }
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Family {
    Manuscript,
    Source,
}
impl Family {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Manuscript => "Literata",
            Self::Source => "IBM Plex Mono",
        }
    }
}
pub(crate) fn source_text(size: f32, text: impl Into<ArcStr>) -> StyledText {
    StyledText {
        text: text.into(),
        size,
        family: Family::Source,
    }
}
impl ViewMarker for StyledText {}
impl<State: 'static> View<State, (), ViewCtx> for StyledText {
    type Element = Pod<Prose>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, ()) {
        let area = TextArea::new_immutable(&self.text)
            .with_word_wrap(true)
            .with_style(StyleProperty::FontSize(self.size))
            .with_style(StyleProperty::FontFamily(self.family.name().into()));
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
        if self.family != previous.family {
            TextArea::insert_style(
                &mut area,
                StyleProperty::FontFamily(self.family.name().into()),
            );
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
