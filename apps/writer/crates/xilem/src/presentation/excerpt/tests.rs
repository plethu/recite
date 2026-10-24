use std::num::NonZeroUsize;
use xilem::masonry::{core::StyleProperty, kurbo::Affine, widgets::Label};
use xilem::masonry::{
    core::{NewWidget, Widget, WidgetId, WindowEvent},
    parley::LineHeight,
    properties::LineBreaking,
    properties::types::CrossAxisAlignment,
    testing::TestHarness,
    theme::default_property_set,
    widgets::Flex,
};

fn harness(text: &str, width: u32, limit: Option<NonZeroUsize>) -> (TestHarness<Flex>, WidgetId) {
    // The harness registers Roboto as a Latin-only fallback and disables system
    // fonts. Name it explicitly so unsupported scripts still get glyph runs;
    // these are text-retention tests, not evidence of Arabic/CJK font shaping.
    let label = NewWidget::new(
        Label::new(text)
            .with_style(StyleProperty::FontFamily("Roboto".into()))
            .with_style(StyleProperty::FontSize(15.))
            .with_style(StyleProperty::LineHeight(LineHeight::Absolute(22.)))
            .with_max_lines(limit),
    )
    .with_props(LineBreaking::WordWrap);
    let id = label.id();
    let root = NewWidget::new(
        Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(label),
    );
    (
        TestHarness::create_with_size(default_property_set(), root, (width, 500)),
        id,
    )
}
fn assert_original(harness: &TestHarness<Flex>, id: WidgetId, text: &str) {
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut value = None;
    while let Some(node) = pending.pop() {
        if node.locate().0.0 == id.to_raw() {
            assert_eq!(
                node.document_range().text(),
                text,
                "text ranges must retain authored text"
            );
            value = node.value();
            break;
        }
        pending.extend(node.children());
    }
    assert_eq!(
        value.as_deref(),
        Some(text),
        "accessible value must retain all authored text"
    );
}
#[test]
fn clamp_keeps_natural_height_and_full_accessible_text() {
    for (text, lines, omitted) in [
        ("One line", 1, false),
        ("One\nTwo", 2, false),
        ("One\nTwo\nThree\nFour", 3, true),
    ] {
        let (harness, id) = harness(text, 240, NonZeroUsize::new(3));
        let label = harness.get_widget_with_id(id).downcast::<Label>().unwrap();
        assert_eq!(label.ctx().border_box().height(), f64::from(lines) * 22.);
        assert_eq!(label.get_debug_text().unwrap().ends_with('…'), omitted);
        assert_original(&harness, id, text);
    }
}
#[test]
fn clamp_reflows_width_text_styles_and_limit_without_stale_cache_results() {
    let text = "A native excerpt with enough words to wrap onto several lines when its card gets narrower.";
    let (mut harness, id) = harness(text, 100, NonZeroUsize::new(2));
    let narrow = harness.get_widget_with_id(id).get_debug_text().unwrap();
    assert!(narrow.ends_with('…'));
    harness.process_window_event(WindowEvent::Resize((600, 500).into()));
    assert_eq!(
        harness.get_widget_with_id(id).get_debug_text().as_deref(),
        Some(text)
    );
    harness.process_window_event(WindowEvent::Resize((100, 500).into()));
    assert_eq!(
        harness.get_widget_with_id(id).get_debug_text().unwrap(),
        narrow
    );
    harness.edit_widget_with_id(id, |mut widget| {
        let mut label = widget.downcast::<Label>();
        Label::set_text(&mut label, "Changed\nMore\nLast");
        Label::set_max_lines(&mut label, NonZeroUsize::new(1));
    });
    assert!(
        harness
            .get_widget_with_id(id)
            .get_debug_text()
            .unwrap()
            .ends_with('…')
    );
    harness.edit_widget_with_id(id, |mut widget| {
        let mut label = widget.downcast::<Label>();
        Label::insert_style(&mut label, StyleProperty::FontSize(25.));
        Label::set_max_lines(&mut label, None);
    });
    assert_eq!(
        harness.get_widget_with_id(id).get_debug_text().as_deref(),
        Some("Changed\nMore\nLast")
    );
}
#[test]
fn shaped_ellipsis_handles_unicode_hard_breaks_and_long_unbroken_text() {
    for text in [
        "a\u{301} e\u{301} i\u{301} o\u{301} u\u{301} ".repeat(20),
        "👩🏾‍🚀 👨‍👩‍👧‍👦 🌈 ".repeat(20),
        "العربية English العربية mixed words ".repeat(20),
        "日本語の長い会話と続きの言葉".repeat(20),
        "One\nTwo\n   \nFour\nFive".into(),
        "unbroken".repeat(1024),
    ] {
        let (ordinary, ordinary_id) = harness(&text, 160, None);
        assert_original(&ordinary, ordinary_id, &text);
        let (harness, id) = harness(&text, 160, NonZeroUsize::new(3));
        let label = harness.get_widget_with_id(id).downcast::<Label>().unwrap();
        assert!(label.ctx().border_box().height() <= 66.01);
        let shapes = label.debug_abbreviation_shapes();
        assert!(label.get_debug_text().unwrap().ends_with('…'));
        assert_original(&harness, id, &text);
        assert!(
            shapes <= 80,
            "bounded candidate shaping regressed: {shapes}"
        );
    }
}
#[test]
fn ordinary_labels_and_unchanged_transforms_do_not_shape_abbreviations() {
    let (ordinary, id) = harness("Unchanged ordinary label", 100, None);
    assert_eq!(
        ordinary
            .get_widget_with_id(id)
            .downcast::<Label>()
            .unwrap()
            .debug_abbreviation_shapes(),
        0
    );
    let (mut harness, id) = harness(
        "A long map excerpt that needs a visual omission marker in a narrow card.",
        100,
        NonZeroUsize::new(2),
    );
    let before = harness
        .get_widget_with_id(id)
        .downcast::<Label>()
        .unwrap()
        .debug_abbreviation_shapes();
    assert!(before > 0);
    for step in 0..32 {
        harness.edit_root_widget(|mut root| {
            root.ctx
                .set_transform(Affine::translate((f64::from(step), 0.)))
        });
        harness.redraw();
    }
    assert_eq!(
        harness
            .get_widget_with_id(id)
            .downcast::<Label>()
            .unwrap()
            .debug_abbreviation_shapes(),
        before,
        "camera transforms must reuse shaped text"
    );
}
