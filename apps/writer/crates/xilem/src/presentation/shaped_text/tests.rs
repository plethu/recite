use super::*;

impl ShapedText {
    pub(crate) fn text(&self) -> &ArcStr {
        &self.text
    }
    pub(crate) fn abbreviation_shapes(&self) -> usize {
        self.abbreviation_shapes
    }
}

#[test]
fn wrapping_measurements_follow_the_inline_extent() {
    let space = Length::const_px(120.);
    let wrap = LineBreaking::WordWrap;
    assert_eq!(max_advance(wrap, true, LenReq::MinContent, None), Some(0.));
    assert_eq!(max_advance(wrap, true, LenReq::MaxContent, None), None);
    assert_eq!(
        max_advance(wrap, true, LenReq::FitContent(space), None),
        Some(120.)
    );
    assert_eq!(
        max_advance(wrap, false, LenReq::MaxContent, Some(space)),
        Some(120.)
    );
    assert_eq!(max_advance(wrap, false, LenReq::MinContent, None), Some(0.));
    for breaking in [LineBreaking::Clip, LineBreaking::Overflow] {
        assert_eq!(
            max_advance(breaking, true, LenReq::FitContent(space), None),
            None
        );
    }
}

#[test]
#[should_panic(expected = "character boundaries")]
fn a_range_cannot_split_a_character() {
    let _ = ShapedText::new("À").with_ranged_styles(vec![(1..2, StyleProperty::Underline(true))]);
}

#[test]
#[should_panic(expected = "single text colour")]
fn a_range_cannot_name_another_brush() {
    let _ = ShapedText::new("word")
        .with_ranged_styles(vec![(0..4, StyleProperty::Brush(BrushIndex(1)))]);
}
