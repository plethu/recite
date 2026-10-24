use super::*;
use freya_testing::prelude::*;
use std::time::Duration;

#[derive(Clone, Copy)]
struct List {
    active: State<Option<usize>>,
    scale: State<f32>,
    unrelated: State<bool>,
    scroll: ScrollController,
}

fn app() -> impl IntoElement {
    use_init_theme(light_theme);
    let list = use_consume::<List>();
    let scale = *list.scale.read();
    let row_height = 84. * scale;
    let _unrelated = *list.unrelated.read();
    use_list_reveal(None, list.active, list.scroll, row_height, 1);
    VirtualScrollView::new_with_data(row_height, |item, height| {
        rect()
            .key(item.index)
            .a11y_alt(format!("Row {}", item.index))
            .height(Size::px(*height))
            .width(Size::fill())
            .child(label().text(format!("Row {}", item.index)))
            .into_element()
    })
    .length(20usize)
    .item_size(row_height)
    .scroll_controller(list.scroll)
    .height(Size::px(200. * scale))
    .width(Size::fill())
}

fn settle(test: &mut TestingRunner) {
    test.poll_n(Duration::from_millis(16), 8);
}

fn assert_visible(test: &TestingRunner, scale: f32) {
    let row = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|rect| rect.accessibility.builder.label() == Some("Row 17"))
                .map(|_| node.layout().area)
        })
        .expect("selected row is mounted");
    assert!(row.min_y() >= 0. && row.max_y() <= 200. * scale, "{row:?}");
    assert_eq!(row.height(), 84. * scale);
}

#[test]
fn scale_changes_reveal_a_near_end_selection_once_without_overriding_manual_scroll() {
    let (mut test, mut list) = TestingRunner::new(
        app,
        (500., 500.).into(),
        |runner| {
            runner.provide_root_context(|| List {
                active: State::create(None),
                scale: State::create(1.),
                unrelated: State::create(false),
                scroll: ScrollController::new(0, 0),
            })
        },
        1.,
    );
    settle(&mut test);
    list.active.set(Some(17));
    settle(&mut test);
    assert_visible(&test, 1.);
    for scale in [2., 1.] {
        list.scroll.scroll_to_y(0);
        list.unrelated.set(true);
        settle(&mut test);
        assert_eq!(<(i32, i32)>::from(list.scroll).1, 0);
        list.scale.set(scale);
        settle(&mut test);
        assert_visible(&test, scale);
        list.scroll.scroll_to_y(0);
        list.unrelated.set(false);
        settle(&mut test);
        assert_eq!(<(i32, i32)>::from(list.scroll).1, 0);
    }
}
