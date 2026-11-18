use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::accesskit::{Rect, Role};
use xilem::masonry::kurbo::Point;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::widgets::Passthrough;
use xilem::style::Style;
use xilem::view::{CrossAxisAlignment, button, flex_col, flex_row, sized_box};

/// How much each route reaches past the height its row counts.
const REACH: f64 = 16.;
/// The routes' strut: a target far taller than the reply's line.
const TARGET: f64 = 60.;

fn reply(n: usize) -> impl WidgetView<usize> + use<> {
    flex_row((
        label(1., format!("Reply {n}")),
        overhang(
            button(
                flex_row((
                    label(1., format!("Route {n}")),
                    sized_box(flex_row(())).width(0.px()).height(TARGET.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center),
                move |pressed: &mut usize| *pressed = n,
            ),
            REACH,
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
}

fn view(_: &mut usize) -> impl WidgetView<usize, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), flex_col((reply(1), reply(2))).gap(REACH.px()))
}

/// The words' boxes by text, and the buttons' targets from the top.
fn bounds(
    mounted: &Mounted<usize, impl WidgetView<usize, Widget = Passthrough>>,
) -> (Vec<(String, Rect)>, Vec<Rect>) {
    let tree = mounted.harness.access_tree();
    let (mut words, mut targets) = (Vec::new(), Vec::new());
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        match (node.role(), node.bounding_box()) {
            (Role::Label, Some(rect)) => words.push((node.value().unwrap_or_default(), rect)),
            (Role::Button, Some(rect)) => targets.push(rect),
            _ => {}
        }
        pending.extend(node.children());
    }
    targets.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    (words, targets)
}

fn word(words: &[(String, Rect)], text: &str) -> Rect {
    words
        .iter()
        .find(|(value, _)| value == text)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("{text} in {words:?}"))
}

#[test]
fn a_tall_target_keeps_its_height_and_baseline_but_reaches_into_the_row_spacing() {
    let mut mounted = Mounted::new(0, view);
    let (words, targets) = bounds(&mounted);
    let [reply, route, next_reply, next_route] =
        ["Reply 1", "Route 1", "Reply 2", "Route 2"].map(|text| word(&words, text));
    // The words share a baseline: the same face at the same size starts at the same height.
    assert!((reply.y0 - route.y0).abs() < 0.5, "{reply:?} {route:?}");
    assert!((next_reply.y0 - next_route.y0).abs() < 0.5);
    let [first, second] = [targets[0], targets[1]];
    let pitch = next_reply.y0 - reply.y0;
    // The reach comes out of the pitch, and the two targets meet without overlapping.
    assert!(first.height() >= TARGET, "{first:?}");
    assert!(
        pitch < first.height() + REACH,
        "pitch {pitch} target {first:?}"
    );
    assert!(second.y0 >= first.y1 - 0.5, "{first:?} {second:?}");
    // A press in the reach above the second row's line lands on its target.
    mounted
        .harness
        .mouse_move(Point::new((second.x0 + second.x1) / 2., second.y0 + 2.));
    mounted.harness.mouse_button_press(None);
    mounted.harness.mouse_button_release(None);
    mounted.settle();
    assert_eq!(mounted.state, 2);
}
