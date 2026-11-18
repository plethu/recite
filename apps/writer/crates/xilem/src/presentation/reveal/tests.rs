use super::*;
use crate::presentation::label;
use crate::tests::support::mounted::Mounted;
use xilem::masonry::accesskit::Role;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::widgets::Passthrough;
use xilem::style::Style;
use xilem::view::{flex_col, portal, sized_box};

fn view(key: &mut usize) -> impl WidgetView<usize, Widget = Passthrough> + use<> {
    crate::keyed::keyed(
        (),
        portal(flex_col((
            sized_box(flex_col(())).height(2_000.px()),
            revealed(*key, label(1., "Current page")),
        ))),
    )
}

/// Where the current page's words sit in the window.
fn top(mounted: &Mounted<usize, impl WidgetView<usize, Widget = Passthrough>>) -> f64 {
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.role() == Role::Label && node.value().as_deref() == Some("Current page") {
            return node.bounding_box().unwrap().y0;
        }
        pending.extend(node.children());
    }
    panic!("no current page");
}

#[test]
fn a_new_key_scrolls_the_child_into_its_viewport() {
    let mut mounted = Mounted::new(0, view);
    assert!(top(&mounted) > 900., "{}", top(&mounted));
    mounted.state = 1;
    mounted.settle();
    let top = top(&mounted);
    assert!((0. ..900.).contains(&top), "{top}");
}
