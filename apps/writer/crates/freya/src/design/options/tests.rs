use super::*;
use crate::design::{ReducedMotion, palette};
use freya_testing::prelude::*;
use std::time::Duration;

fn specimen() -> impl IntoElement {
    use_init_theme(|| palette::theme(true));
    let mut selected = use_state(|| 0);
    let mut reduced = use_state(|| false);
    use_provide_context(|| ReducedMotion(reduced));
    rect()
        .child(Segments {
            shortcuts: None,
            name: "View".into(),
            labels: ["Map".into(), "Source".into()],
            ids: [use_a11y(), use_a11y()],
            selected: *selected.read(),
            vim: false,
            change: EventHandler::new(move |next| selected.set(next)),
            width: Size::px(240.),
        })
        .child(
            Button::new()
                .flat()
                .named("Reduce")
                .on_press(move |_| reduced.set(true))
                .child("Reduce"),
        )
}

fn indicator(test: &TestingRunner) -> Area {
    test.find(|node, element| {
        Rect::try_downcast(element)
            .filter(|rect| {
                rect.style.background.as_color() == Some(palette::Palette::new(true).selection)
            })
            .map(|_| node.layout().area)
    })
    .expect("selection plate")
}

fn button(test: &TestingRunner, name: &str) -> Area {
    test.find(|node, element| {
        Rect::try_downcast(element)
            .filter(|rect| rect.accessibility.builder.label() == Some(name))
            .map(|_| node.layout().area)
    })
    .expect("button")
}

#[test]
fn selection_travels_reverses_and_respects_reduced_motion_without_moving_targets() {
    let mut test = TestingRunner::new(specimen, Size2D::new(300., 120.), |_| {}, 1.).0;
    test.poll_n(Duration::from_millis(16), 4);
    let start = indicator(&test);
    let source = button(&test, "View: Source");
    let map = button(&test, "View: Map");
    assert_eq!(
        start, map,
        "the plate sits directly beneath the selected button"
    );
    test.click_cursor(source.center().to_f64());
    test.poll_n(Duration::from_millis(16), 3);
    let middle = indicator(&test);
    assert!(
        middle.min_x() > start.min_x() + start.width() * 0.5,
        "most travel happens early"
    );
    assert!(middle.min_x() < start.min_x() + start.width());
    assert_eq!(source, button(&test, "View: Source"));
    assert_eq!(map, button(&test, "View: Map"));
    test.click_cursor(map.center().to_f64());
    let reversed = indicator(&test);
    assert!(
        (reversed.min_x() - middle.min_x()).abs() < 1.,
        "retarget from the visible frame"
    );
    test.poll_n(Duration::from_millis(16), 18);
    assert_eq!(indicator(&test), start);
    test.click_cursor(button(&test, "Reduce").center().to_f64());
    test.click_cursor(source.center().to_f64());
    test.poll_n(Duration::from_millis(1), 2);
    assert!((indicator(&test).min_x() - start.min_x() - start.width()).abs() < 1.);
}

#[test]
fn segment_captions_remain_centered_inside_their_hit_targets() {
    let mut test = TestingRunner::new(specimen, (400., 200.).into(), |_| {}, 1.).0;
    test.poll_n(Duration::from_millis(16), 5);
    for title in ["Map", "Source"] {
        let button = test
            .find(|node, element| {
                Rect::try_downcast(element)
                    .filter(|r| {
                        r.accessibility.builder.label() == Some(format!("View: {title}").as_str())
                    })
                    .map(|_| node.layout().area)
            })
            .expect("segment");
        let caption = test
            .find(|node, element| {
                Label::try_downcast(element)
                    .filter(|label| label.text == title)
                    .map(|_| node.layout().area)
            })
            .expect("caption");
        assert!((button.center().x - caption.center().x).abs() < 1.);
    }
}
