use super::*;
use crate::presentation::ShapedText;
use crate::presentation::Theme;
use xilem::masonry::{accesskit::Role, core::Widget, testing::TestHarness};

#[test]
fn highlighted_search_results_keep_complete_native_text() {
    let text = "The café courier takes tram 39.";
    let view = manuscript_search(20., text.into(), "CAFÉ 39".into());
    let label = ShapedText::new(text)
        .with_ranged_styles(view.styles())
        .prepare()
        .with_props(LineBreaking::WordWrap);
    let harness = TestHarness::create_with_size(
        Theme::new(&recite_config::WriterConfig::default()).initial_properties(),
        label,
        (320, 400),
    );
    let (actual, emphasis) = read(&harness);
    assert_eq!(actual, text);
    assert!(emphasis.iter().any(|part| part.contains("café")));
    assert!(emphasis.iter().any(|part| part.contains("39")));
}

fn read(harness: &TestHarness<ShapedText>) -> (String, Vec<String>) {
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut text = String::new();
    let mut emphasis = Vec::new();
    while let Some(node) = pending.pop() {
        if node.role() == Role::TextRun {
            let value = node.value().unwrap_or_default();
            text.push_str(&value);
            if node.font_weight().is_some_and(|weight| weight >= 650.) && node.underline().is_some()
            {
                emphasis.push(value);
            }
        }
        let mut children: Vec<_> = node.children().collect();
        children.reverse();
        pending.extend(children);
    }
    (text, emphasis)
}

#[test]
fn ranged_styles_preserve_wrapped_text_order_and_clear_on_replacement() {
    let text = "À neuf le tram part. Le dernier tram traverse le port.";
    for size in [20., 40.] {
        let widget = ShapedText::new(text)
            .with_style(StyleProperty::FontSize(size))
            .with_ranged_styles(vec![
                (3..8, StyleProperty::FontWeight(FontWeight::BOLD)),
                (3..8, StyleProperty::Underline(true)),
            ])
            .prepare()
            .with_props(LineBreaking::WordWrap);
        let mut harness = TestHarness::create_with_size(
            Theme::new(&recite_config::WriterConfig::default()).initial_properties(),
            widget,
            (320, 400),
        );
        let (actual, emphasis) = read(&harness);
        assert_eq!(actual, text);
        assert!(emphasis.concat().contains("neuf"), "{emphasis:?}");
        harness.edit_root_widget(|mut widget| ShapedText::set_ranged_styles(&mut widget, vec![]));
        harness.redraw();
        assert_eq!(read(&harness), (text.to_owned(), vec![]));
        harness.edit_root_widget(|mut widget| {
            ShapedText::set_ranged_styles(
                &mut widget,
                vec![(3..8, StyleProperty::Underline(true))],
            );
            ShapedText::set_text(&mut widget, "Changé.");
        });
        harness.redraw();
        assert_eq!(read(&harness), ("Changé.".to_owned(), vec![]));
    }
}
