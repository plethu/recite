//! Reveal a newly focused dialog control once; manual scrolling keeps ownership afterward.
use freya::prelude::*;

#[derive(Clone, Copy)]
struct Viewport {
    scroll: ScrollController,
    area: State<Option<Area>>,
}

#[derive(Clone, PartialEq)]
pub(super) struct Body(pub Element);
impl Component for Body {
    fn render(&self) -> impl IntoElement {
        let scroll = use_scroll_controller(ScrollConfig::default);
        let mut area = use_state(|| None);
        use_provide_context(|| Viewport { scroll, area });
        ScrollView::new_controlled(scroll)
            .height(Size::auto())
            .width(Size::fill())
            .max_height(Size::window_percent(60.))
            .on_sized(move |event: Event<SizedEventData>| area.set_if_modified(Some(event.area)))
            .child(self.0.clone())
    }
}

pub(super) fn use_reveal(id: AccessibilityId, area: State<Option<Area>>) {
    let viewport = use_try_consume::<Viewport>();
    let mut revealed = use_state(|| false);
    use_after_side_effect(move || {
        if !id.is_focused() {
            revealed.set_if_modified(false);
            return;
        }
        let Some(mut viewport) = viewport else {
            return;
        };
        let (Some(target), Some(visible)) = (*area.read(), *viewport.area.read()) else {
            return;
        };
        if *revealed.peek() {
            return;
        }
        // Freya already scrolls fully clipped controls into view. Let that
        // wheel event settle before correcting any remaining partial clipping.
        // Applying both deltas against the old layout scrolls past the target.
        if !visible.intersects(&target) {
            return;
        }
        revealed.set(true);
        let delta = if target.min_y() < visible.min_y() {
            visible.min_y() - target.min_y()
        } else if target.max_y() > visible.max_y() {
            visible.max_y() - target.max_y()
        } else {
            0.
        };
        if delta.abs() >= 1. {
            let (_, y): (i32, i32) = viewport.scroll.into();
            viewport.scroll.scroll_to_y(y + delta.round() as i32);
        }
    });
}
